mod try_load;

extern crate proc_macro;

use asr::{print_message, settings, timer};
pub use paste::paste;
use std::collections::HashSet;

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
    print_message(&format!("trying to split {key}"));
    let settings_map = settings::Map::load();

    if completed_splits.contains(key) {
        print_message(&format!("-> {key} already split"));
        return false;
    }

    let value = settings_map.get(key);

    if value.is_none() {
        print_message(&format!("-> {key} not found"));
        return false;
    }

    let value = value.unwrap().get_bool();
    if value.is_none() {
        print_message(&format!("-> {key} not a bool"));
        return false;
    }

    if !value.unwrap() {
        print_message(&format!("-> {key} set to false"));
        return false;
    }

    if !completed_splits.insert(key.to_owned()) {
        print_message(&format!("-> {key} already split"));
        return false;
    }

    print_message(&format!("-> {key} split!"));
    timer::split();

    true
}
