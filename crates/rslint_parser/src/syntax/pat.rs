use super::expr::{assign_expr, identifier_name, identifier_reference, object_prop_name};
use crate::{SyntaxKind::*, *};

pub fn pattern(p: &mut Parser) -> Option<CompletedMarker> {
    Some(match p.cur() {
        T![ident] | T![yield] | T![await] => {
            let m = p.start();
            binding_identifier(p);
            m.complete(p, SINGLE_PATTERN)
        }
        T!['['] => array_binding_pattern(p),
        T!['{'] if p.state.allow_object_expr => object_binding_pattern(p),
        _ => {
            let err = p
                .err_builder("Expected an identifier or pattern, but found none")
                .primary(p.cur_tok().range, "");
            let mut ts = token_set![T![ident], T![yield], T![await], T!['['],];
            if p.state.allow_object_expr {
                ts = ts.union(token_set![T!['{']]);
            }
            p.err_recover(err, ts, false);
            return None;
        }
    })
}

pub fn opt_binding_identifier(p: &mut Parser) -> Option<CompletedMarker> {
    const BINDING_IDENTS: TokenSet = token_set![T![ident], T![yield], T![await]];

    if p.at_ts(BINDING_IDENTS) {
        binding_identifier(p)
    } else {
        None
    }
}

// test_err binding_identifier_invalid
// async () => { let await = 5; }
// function *foo() {
//    let yield = 5;
// }
// let eval = 5;
pub fn binding_identifier(p: &mut Parser) -> Option<CompletedMarker> {
    if p.at(T![yield]) && p.state.in_generator {
        let err = p
            .err_builder("Illegal use of `yield` as an identifier in generator function")
            .primary(p.cur_tok().range, "");

        p.error(err);
    }

    if p.at(T![await]) && p.state.in_async {
        let err = p
            .err_builder("Illegal use of `await` as an identifier in an async context")
            .primary(p.cur_tok().range, "");

        p.error(err);
    }

    if p.state.strict.is_some() && (p.cur_src() == "eval" || p.cur_src() == "arguments") {
        let err = p
            .err_builder(&format!(
                "Illegal use of `{}` as an identifier in strict mode",
                p.cur_src()
            ))
            .primary(p.cur_tok().range, "");

        p.error(err);
    }

    let mut m = identifier_reference(p)?;
    m.change_kind(p, NAME);
    Some(m)
}

pub fn binding_element(p: &mut Parser) -> Option<CompletedMarker> {
    let left = pattern(p);

    if p.at(T![=]) {
        let m = left.map(|m| m.precede(p)).unwrap_or_else(|| p.start());
        p.bump_any();

        assign_expr(p);
        return Some(m.complete(p, ASSIGN_PATTERN));
    }

    left
}

pub fn array_binding_pattern(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.expect(T!['[']);

    while !p.at(EOF) && !p.at(T![']']) {
        if p.eat(T![,]) {
            continue;
        }
        if p.at(T![...]) {
            let m = p.start();
            p.bump_any();

            pattern(p);

            m.complete(p, REST_PATTERN);
            break;
        } else if binding_element(p).is_none() {
            p.err_recover_no_err(
                token_set![T![await], T![ident], T![yield], T![:], T![=], T![']']],
                false,
            );
        }
        if !p.at(T![']']) {
            p.expect(T![,]);
        }
    }

    p.expect(T![']']);
    m.complete(p, ARRAY_PATTERN)
}

pub fn object_binding_pattern(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.expect(T!['{']);
    let mut first = true;

    while !p.at(EOF) && !p.at(T!['}']) {
        if first {
            first = false;
        } else {
            p.expect(T![,]);
            if p.at(T!['}']) {
                break;
            }
        }

        if p.at(T![...]) {
            let m = p.start();
            p.bump_any();

            pattern(p);
            m.complete(p, REST_PATTERN);
            break;
        }

        object_binding_prop(p);
    }
    p.expect(T!['}']);
    m.complete(p, OBJECT_PATTERN)
}

// test object_binding_prop
// let { default: foo, bar } = {}
// let { foo = bar, baz } = {}
fn object_binding_prop(p: &mut Parser) -> Option<CompletedMarker> {
    let m = p.start();
    let name = if (p.cur().is_keyword() || p.cur() == T![ident]) && p.nth(1) == T![:] {
        identifier_name(p)
    } else {
        object_prop_name(p, true)
    };

    if p.eat(T![:]) {
        binding_element(p);
        return Some(m.complete(p, KEY_VALUE_PATTERN));
    }

    if name.is_none() {
        p.err_recover_no_err(
            token_set![T![await], T![ident], T![yield], T![:], T![=], T!['}']],
            false,
        );
        return None;
    }

    if name.map(|x| x.kind()) != Some(NAME) {
        let err = p
            .err_builder("Expected an identifier for a pattern, but found none")
            .primary(name.unwrap().range(p), "");

        p.error(err);
        return None;
    }

    if p.eat(T![=]) {
        assign_expr(p);
        name.unwrap().change_kind(p, SINGLE_PATTERN);
        Some(m.complete(p, ASSIGN_PATTERN))
    } else {
        Some(name?.precede(p).complete(p, SINGLE_PATTERN))
    }
}
