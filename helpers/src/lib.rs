extern crate proc_macro;

use asr::{settings, timer};
use std::collections::HashSet;
pub use paste::paste;

#[macro_export]
macro_rules! impl_auto_splitter_state {
    ($watchers:ident {
        $($field:ident : Watcher<$ty:ty>),+ $(,)?
    }) => {
        paste! {
            struct [<$watchers __State>] {
                $($field: $ty,)+
            }

            #[derive(Default)]
            struct $watchers {
                $($field: Watcher<$ty>,)+
            }

            impl $watchers {
                fn to_states(&self) -> Option<([<$watchers __State>], [<$watchers __State>])> {
                    $(let $field = self.$field.pair.as_ref()?;)+

                    let current = [<$watchers __State>] {
                        $($field: $field.current.to_owned(),)+
                    };

                    #[cfg(debug_assertions)]
                    {
                        $(timer::set_variable(stringify!($field), &format!("{:#?}", current.$field));)+
                    }

                    Some((
                        [<$watchers __State>] {
                            $($field: $field.old.to_owned(),)+
                        },
                        current,
                    ))
                }
            }
        }
    };
}

pub fn split(key: &String, completed_splits: &mut HashSet<String>) -> bool {
    let settings_map = settings::Map::load();

    if completed_splits.contains(key) {
        return false;
    }

    return if settings_map
        .get(key)
        .unwrap_or(settings::Value::from(false))
        .get_bool()
        .unwrap_or_default()
    {
        completed_splits.insert(key.to_owned());
        timer::split();
        true
    } else {
        false
    };
}