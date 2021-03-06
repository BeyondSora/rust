// A pass that checks to make sure private fields and methods aren't used
// outside their scopes.

use /*mod*/ syntax::ast;
use /*mod*/ syntax::visit;
use syntax::ast_map;
use syntax::ast::{def_variant, expr_field, expr_struct, expr_unary, ident,
                  item_class};
use syntax::ast::{item_impl, item_trait, item_enum, local_crate, node_id,
                  pat_struct};
use syntax::ast::{private, provided, required};
use syntax::ast_map::{node_item, node_method};
use syntax::ast_util::{has_legacy_export_attr, is_local,
                       visibility_to_privacy, Private, Public};
use ty::{ty_class, ty_enum};
use typeck::{method_map, method_origin, method_param, method_self};
use typeck::{method_static, method_trait};

use core::util::ignore;
use dvec::DVec;

fn check_crate(tcx: ty::ctxt, method_map: &method_map, crate: @ast::crate) {
    let privileged_items = @DVec();
    let legacy_exports = has_legacy_export_attr(crate.node.attrs);

    // Adds structs that are privileged to this scope.
    let add_privileged_items = |items: &[@ast::item]| {
        let mut count = 0;
        for items.each |item| {
            match item.node {
                item_class(*) | item_trait(*) | item_impl(*)
                | item_enum(*) => {
                    privileged_items.push(item.id);
                    count += 1;
                }
                _ => {}
            }
        }
        count
    };

    // Checks that an enum variant is in scope
    let check_variant = |span, enum_id| {
        let variant_info = ty::enum_variants(tcx, enum_id)[0];
        let parental_privacy = if is_local(enum_id) {
            let parent_vis = ast_map::node_item_query(tcx.items, enum_id.node,
                                   |it| { it.vis },
                                   ~"unbound enum parent when checking \
                                    dereference of enum type");
            visibility_to_privacy(parent_vis, legacy_exports)
        }
        else {
            // WRONG
            Public
        };
        debug!("parental_privacy = %?", parental_privacy);
        debug!("vis = %?, priv = %?, legacy_exports = %?",
               variant_info.vis,
               visibility_to_privacy(variant_info.vis, legacy_exports),
               legacy_exports);
        // inherited => privacy of the enum item
        if visibility_to_privacy(variant_info.vis,
                                 parental_privacy == Public) == Private {
            tcx.sess.span_err(span,
                ~"can only dereference enums \
                  with a single, public variant");
        }
    };

    // Checks that a private field is in scope.
    let check_field = |span, id, ident| {
        let fields = ty::lookup_class_fields(tcx, id);
        for fields.each |field| {
            if field.ident != ident { loop; }
            if field.vis == private {
                tcx.sess.span_err(span, fmt!("field `%s` is private",
                                             *tcx.sess.parse_sess.interner
                                                 .get(ident)));
            }
            break;
        }
    };

    // Checks that a private method is in scope.
    let check_method = |span, origin: &method_origin| {
        match *origin {
            method_static(method_id) => {
                if method_id.crate == local_crate {
                    match tcx.items.find(method_id.node) {
                        Some(node_method(method, impl_id, _)) => {
                            if method.vis == private &&
                                    (impl_id.crate != local_crate ||
                                     !privileged_items
                                     .contains(&(impl_id.node))) {
                                tcx.sess.span_err(span,
                                                  fmt!("method `%s` is \
                                                        private",
                                                       *tcx.sess
                                                           .parse_sess
                                                           .interner
                                                           .get(method
                                                                .ident)));
                            }
                        }
                        Some(_) => {
                            tcx.sess.span_bug(span, ~"method wasn't \
                                                      actually a method?!");
                        }
                        None => {
                            tcx.sess.span_bug(span, ~"method not found in \
                                                      AST map?!");
                        }
                    }
                } else {
                    // XXX: External crates.
                }
            }
            method_param({trait_id: trait_id, method_num: method_num, _}) |
            method_trait(trait_id, method_num, _) |
            method_self(trait_id, method_num) => {
                if trait_id.crate == local_crate {
                    match tcx.items.find(trait_id.node) {
                        Some(node_item(item, _)) => {
                            match item.node {
                                item_trait(_, _, methods) => {
                                    if method_num >= methods.len() {
                                        tcx.sess.span_bug(span, ~"method \
                                                                  number \
                                                                  out of \
                                                                  range?!");
                                    }
                                    match methods[method_num] {
                                        provided(method)
                                             if method.vis == private &&
                                             !privileged_items
                                             .contains(&(trait_id.node)) => {
                                            tcx.sess.span_err(span,
                                                              fmt!("method
                                                                    `%s` \
                                                                    is \
                                                                    private",
                                                                   *tcx
                                                                   .sess
                                                                   .parse_sess
                                                                   .interner
                                                                   .get
                                                                   (method
                                                                    .ident)));
                                        }
                                        provided(_) | required(_) => {
                                            // Required methods can't be
                                            // private.
                                        }
                                    }
                                }
                                _ => {
                                    tcx.sess.span_bug(span, ~"trait wasn't \
                                                              actually a \
                                                              trait?!");
                                }
                            }
                        }
                        Some(_) => {
                            tcx.sess.span_bug(span, ~"trait wasn't an \
                                                      item?!");
                        }
                        None => {
                            tcx.sess.span_bug(span, ~"trait item wasn't \
                                                      found in the AST \
                                                      map?!");
                        }
                    }
                } else {
                    // XXX: External crates.
                }
            }
        }
    };

    let visitor = visit::mk_vt(@{
        visit_mod: |the_module, span, node_id, method_map, visitor| {
            let n_added = add_privileged_items(the_module.items);

            visit::visit_mod(the_module, span, node_id, method_map, visitor);

            // FIXME #4054: n_added gets corrupted without this log statement
            debug!("%i", n_added);

            for n_added.times {
                ignore(privileged_items.pop());
            }
        },
        visit_expr: |expr, method_map: &method_map, visitor| {
            match expr.node {
                expr_field(base, ident, _) => {
                    match ty::get(ty::expr_ty(tcx, base)).sty {
                        ty_class(id, _)
                        if id.crate != local_crate ||
                           !privileged_items.contains(&(id.node)) => {
                            match method_map.find(expr.id) {
                                None => {
                                    debug!("(privacy checking) checking \
                                            field access");
                                    check_field(expr.span, id, ident);
                                }
                                Some(entry) => {
                                    debug!("(privacy checking) checking \
                                            impl method");
                                    check_method(expr.span, &entry.origin);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                expr_struct(_, fields, _) => {
                    match ty::get(ty::expr_ty(tcx, expr)).sty {
                        ty_class(id, _) => {
                            if id.crate != local_crate ||
                                    !privileged_items.contains(&(id.node)) {
                                for fields.each |field| {
                                        debug!("(privacy checking) checking \
                                                field in struct literal");
                                    check_field(expr.span, id,
                                                field.node.ident);
                                }
                            }
                        }
                        ty_enum(id, _) => {
                            if id.crate != local_crate ||
                                    !privileged_items.contains(&(id.node)) {
                                match tcx.def_map.get(expr.id) {
                                    def_variant(_, variant_id) => {
                                        for fields.each |field| {
                                                debug!("(privacy checking) \
                                                        checking field in \
                                                        struct variant \
                                                        literal");
                                            check_field(expr.span, variant_id,
                                                        field.node.ident);
                                        }
                                    }
                                    _ => {
                                        tcx.sess.span_bug(expr.span,
                                                          ~"resolve didn't \
                                                            map enum struct \
                                                            constructor to a \
                                                            variant def");
                                    }
                                }
                            }
                        }
                        _ => {
                            tcx.sess.span_bug(expr.span, ~"struct expr \
                                                           didn't have \
                                                           struct type?!");
                        }
                    }
                }
                expr_unary(ast::deref, operand) => {
                    // In *e, we need to check that if e's type is an
                    // enum type t, then t's first variant is public or
                    // privileged. (We can assume it has only one variant
                    // since typeck already happened.)
                    match ty::get(ty::expr_ty(tcx, operand)).sty {
                        ty_enum(id, _) => {
                            if id.crate != local_crate ||
                                !privileged_items.contains(&(id.node)) {
                                check_variant(expr.span, id);
                            }
                        }
                        _ => { /* No check needed */ }
                    }
                }
                _ => {}
            }

            visit::visit_expr(expr, method_map, visitor);
        },
        visit_pat: |pattern, method_map, visitor| {
            match pattern.node {
                pat_struct(_, fields, _) => {
                    match ty::get(ty::pat_ty(tcx, pattern)).sty {
                        ty_class(id, _) => {
                            if id.crate != local_crate ||
                                    !privileged_items.contains(&(id.node)) {
                                for fields.each |field| {
                                        debug!("(privacy checking) checking \
                                                struct pattern");
                                    check_field(pattern.span, id,
                                                field.ident);
                                }
                            }
                        }
                        ty_enum(enum_id, _) => {
                            if enum_id.crate != local_crate ||
                                    !privileged_items.contains(
                                        &enum_id.node) {
                                match tcx.def_map.find(pattern.id) {
                                    Some(def_variant(_, variant_id)) => {
                                        for fields.each |field| {
                                            debug!("(privacy checking) \
                                                    checking field in \
                                                    struct variant pattern");
                                            check_field(pattern.span,
                                                        variant_id,
                                                        field.ident);
                                        }
                                    }
                                    _ => {
                                        tcx.sess.span_bug(pattern.span,
                                                          ~"resolve didn't \
                                                            map enum struct \
                                                            pattern to a \
                                                            variant def");
                                    }
                                }
                            }
                        }
                        _ => {
                            tcx.sess.span_bug(pattern.span,
                                              ~"struct pattern didn't have \
                                                struct type?!");
                        }
                    }
                }
                _ => {}
            }

            visit::visit_pat(pattern, method_map, visitor);
        },
        .. *visit::default_visitor()
    });
    visit::visit_crate(*crate, method_map, visitor);
}

