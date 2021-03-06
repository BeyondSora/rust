// xfail-test - this isn't really a test.

 {

// select!
macro_rules! select_if (

    {
        $index:expr,
        $count:expr
    } => {
        fail
    };

    {
        $index:expr,
        $count:expr,
        $port:path => [
            $(type_this $message:path$(($(x $x: ident),+))dont_type_this*
              -> $next:ident => { move $e:expr }),+
        ]
        $(, $ports:path => [
            $(type_this $messages:path$(($(x $xs: ident),+))dont_type_this*
              -> $nexts:ident => { move $es:expr }),+
        ] )*
    } => {
        if $index == $count {
            match move pipes::try_recv($port) {
              $(Some($message($($(move $x,)+)* move next)) => {
                let $next = move next;
                move $e
              })+
              _ => fail
            }
        } else {
            select_if!(
                $index,
                $count + 1
                $(, $ports => [
                    $(type_this $messages$(($(x $xs),+))dont_type_this*
                      -> $nexts => { move $es }),+
                ])*
            )
        }
    };
)

macro_rules! select (
    {
        $( $port:path => {
            $($message:path$(($($x: ident),+))dont_type_this*
              -> $next:ident $e:expr),+
        } )+
    } => {
        let index = pipes::selecti([$(($port).header()),+]);
        select_if!(index, 0 $(, $port => [
            $(type_this $message$(($(x $x),+))dont_type_this* -> $next => { move $e }),+
        ])+)
    }
)

}
