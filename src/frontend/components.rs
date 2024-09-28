use maud::{html, Markup};

pub fn text_input(id: &str, label: &str, placeholder: &str, required: bool) ->  Markup {
    html! {
        div {
            label .block .mb-2 .text-sm .font-medium .text-gray-900 ."dark:text-white" for=(id) { (label) }
            input #(id) type="text" .bg-gray-50 .border .border-gray-300 .text-gray-900
                .text-sm .rounded-lg ."focus:ring-blue-500" ."focus:border-blue-500"
                .block .w-full ."p-2.5" ."dark:bg-gray-700" ."dark:border-gray-600" ."dark:placeholder-gray-400"
                ."dark:text-white" ."dark:focus:ring-blue-500" ."dark:focus:border-blue-500"
                placeholder=(placeholder) name=(id) required[required] { }
        }
    }
}

pub fn textarea_input(id: &str, label: &str, placeholder: &str, required: bool) -> Markup {
    html! {
        label for=(id) .block."mb-2".text-sm.font-medium.text-gray-900
            ."dark:text-white" { (label) }
        textarea #(id) rows="4"
            class="
                block p-2.5 w-full text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300
                focus:ring-blue-500 focus:border-blue-500
                dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white
                dark:focus:ring-blue-500 dark:focus:border-blue-500
                "
            placeholder=(placeholder) name=(id) required[required] { }
    }
}

/// Describes one selectable element of a radio
pub struct RadioElement<'a> {
    pub name: &'a str,
    pub default: bool,
}

impl<'a> From<(&'a str, bool)> for RadioElement<'a> {
    fn from(value: (&'a str, bool)) -> Self {
        Self { name: value.0, default: value.1 }
    }
}

pub fn radio<'a>(id: &str, elements: impl IntoIterator<Item = impl Into<RadioElement<'a>>>) -> Markup {
    html! {
        @for (sub_id, element) in elements.into_iter().map(Into::into).enumerate() {
            @let sub_id = format!("{id}-{sub_id}");

            div .flex.items-center.mb-2 {
                input #(sub_id) checked[element.default] type="radio" value=(element.name) name=(id)
                    class="
                    w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600
                    dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600
                    " {}
                label for=(sub_id) class="ms-2 text-sm font-medium text-gray-900 dark:text-gray-300" { (element.name) }
            }
        }
    }
}

pub fn checkbox(id: &str, name: &str) -> Markup {
    html! {
        div .flex.items-center.mb-4 {
            input #(id) type="checkbox" name=(id) value="true"
                class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300
                rounded focus:ring-blue-500
                dark:focus:ring-blue-600 dark:ring-offset-gray-800
                focus:ring-2 dark:bg-gray-700 dark:border-gray-600" {}
            label for=(id)
                class="ms-2 text-sm font-medium text-gray-900 dark:text-gray-300"
                { (name) }
        }
    }
}

#[allow(unused)]
pub mod button {
    pub const DEFAULT: &str = " text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800 ";
    pub const ALTERNATIVE: &str = " py-2.5 px-5 me-2 mb-2 text-sm font-medium text-gray-900 focus:outline-none bg-white rounded-lg border border-gray-200 hover:bg-gray-100 hover:text-blue-700 focus:z-10 focus:ring-4 focus:ring-gray-100 dark:focus:ring-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-600 dark:hover:text-white dark:hover:bg-gray-700 ";
    pub const DARK: &str = " text-white bg-gray-800 hover:bg-gray-900 focus:outline-none focus:ring-4 focus:ring-gray-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-gray-800 dark:hover:bg-gray-700 dark:focus:ring-gray-700 dark:border-gray-700 ";
    pub const LIGHT: &str = " text-gray-900 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:hover:border-gray-600 dark:focus:ring-gray-700 ";
    pub const GREEN: &str = " focus:outline-none text-white bg-green-700 hover:bg-green-800 focus:ring-4 focus:ring-green-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-green-600 dark:hover:bg-green-700 dark:focus:ring-green-800 ";
    pub const RED: &str = " focus:outline-none text-white bg-red-700 hover:bg-red-800 focus:ring-4 focus:ring-red-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-red-600 dark:hover:bg-red-700 dark:focus:ring-red-900";
    pub const YELLOW: &str = " focus:outline-none text-white bg-yellow-400 hover:bg-yellow-500 focus:ring-4 focus:ring-yellow-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:focus:ring-yellow-900 ";
    pub const PURPLE: &str = " focus:outline-none text-white bg-purple-700 hover:bg-purple-800 focus:ring-4 focus:ring-purple-300 font-medium rounded-lg text-sm px-5 py-2.5 mb-2 dark:bg-purple-600 dark:hover:bg-purple-700 dark:focus:ring-purple-900 ";
}

pub const CARD: &str = "
    block bg-white border border-gray-200 rounded-lg shadow
    text-white
    dark:bg-gray-800 dark:border-gray-700
";
