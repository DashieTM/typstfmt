use super::*;

#[instrument(skip_all)]
pub(crate) fn format_args(parent: &LinkedNode, children: &[String], ctx: &mut Ctx) -> String {
    if children.iter().any(|c| c.contains('\n')) {
        return format_args_breaking(parent, children, ctx);
    }

    let mut res = format_args_tight(parent, children, ctx);
    let number_of_args = parent
        .children()
        .filter_map(|node| {
            if [Comma, Space, LeftParen, RightParen].contains(&node.kind()) {
                None
            } else {
                Some(node)
            }
        })
        .count();

    if number_of_args <= 1 {
        return res;
    }

    if utils::max_line_length(&res) >= ctx.config.max_line_length {
        res = format_args_breaking(parent, children, ctx);
        return res;
    }
    res
}

pub(crate) fn format_args_tight(
    parent: &LinkedNode<'_>,
    children: &[String],
    ctx: &mut Ctx,
) -> String {
    let mut res = String::new();
    for (s, node) in children.iter().zip(parent.children()) {
        match node.kind() {
            Space => {}
            Comma => {
                if utils::next_is_ignoring(&node, RightParen, &[Space]) {
                    // not putting the comma in would result in a parenthesized expression, not an array
                    // "(a,) != (a)"
                    if node.parent_kind() == Some(Array) {
                        ctx.push_raw_in(",", &mut res)
                    }
                    // don't print
                } else {
                    ctx.push_raw_in(s, &mut res);
                    ctx.push_in(" ", &mut res);
                }
            }
            _ => {
                ctx.push_raw_in(s, &mut res);
            }
        }
    }
    res
}

pub(crate) fn format_args_breaking(
    parent: &LinkedNode<'_>,
    children: &[String],
    ctx: &mut Ctx,
) -> String {
    let mut res = String::new();
    let mut missing_trailing = !(parent.kind() == Parenthesized);
    for (s, node) in children.iter().zip(parent.children()) {
        let is_last = utils::next_is_ignoring(&node, RightParen, &[Space]);
        match node.kind() {
            LeftParen => {
                res.push_str(s);
                res.push('\n');
                res.push_str(&ctx.get_indent());
                if let Some(next) = utils::get_next_ignoring(&node, &[Space]) {
                    if [LineComment, BlockComment].contains(&next.kind()) {
                        ctx.push_raw_in(" ", &mut res);
                        res.push_str(next.text());
                        res.push('\n');
                        res.push_str(&ctx.get_indent());
                    }
                }
            }
            RightParen => {
                if parent.kind() == Parenthesized {
                    // lets check for comment!
                    let prev = node.prev_sibling().unwrap();
                    let next = utils::get_next_ignoring(&prev, &[Space]);
                    let next_is_comment = next
                        .as_ref()
                        .is_some_and(|n| [LineComment, BlockComment].contains(&n.kind()));
                    if next_is_comment {
                        ctx.push_raw_in(" ", &mut res);
                        ctx.push_raw_indent(next.unwrap().text(), &mut res);
                    }
                    // no trailing comma we don't have a newline!
                    res.push('\n');
                }
                res.push_str(s);
            }
            LineComment | BlockComment => {
                // this will be dealt with in comma and leftParen.
                // except
                if is_last && missing_trailing {
                    res.push_str(", ");
                    ctx.push_raw_indent(s, &mut res);
                    res.push('\n');
                }
            }
            Space => {}
            // handles trailing comma
            // handles Line comment
            Comma => {
                missing_trailing = false;
                // print the last comma but don't indent
                let is_last_comma = utils::find_next(&node, &|x| x.kind() == Comma).is_none();
                let is_trailing =
                    utils::next_is_ignoring(&node, RightParen, &[Space, LineComment, BlockComment]);

                let next = utils::get_next_ignoring(&node, &[Space]);
                let next_is_comment = next
                    .as_ref()
                    .is_some_and(|n| [LineComment, BlockComment].contains(&n.kind()));

                if is_last_comma && is_trailing {
                    // no indent
                    ctx.push_raw_in(s, &mut res);
                    if next_is_comment {
                        ctx.push_raw_in(" ", &mut res);
                        ctx.push_raw_indent(next.unwrap().text(), &mut res);
                    }
                    ctx.push_raw_in("\n", &mut res);
                } else {
                    if is_last_comma && !is_trailing {
                        missing_trailing = true;
                    }
                    res.push_str(s);
                    if next_is_comment {
                        ctx.push_raw_in(" ", &mut res);
                        ctx.push_raw_indent(next.unwrap().text(), &mut res);
                    }
                    res.push('\n');
                    res.push_str(&ctx.get_indent());
                }
            }
            _ => {
                ctx.push_raw_indent(s, &mut res);
                if is_last && missing_trailing {
                    ctx.push_raw_in(",\n", &mut res)
                }
            }
        }
    }
    res
}