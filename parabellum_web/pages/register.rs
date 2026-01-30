use dioxus::prelude::*;
use rust_i18n::t;

/// Register page component
#[component]
pub fn RegisterPage(
    csrf_token: String,
    username_value: String,
    email_value: String,
    selected_tribe: String,
    selected_quadrant: String,
    error: Option<String>,
) -> Element {
    rsx! {
        div { class: "flex flex-col items-center w-full px-4",
            div { class: "login-box",
                h2 { class: "text-xl font-bold text-gray-700 mb-6 text-center",
                    "{t!(\"register.form.title\")}"
                }
                if let Some(err) = error {
                    p { class: "mb-3 text-center text-red-600 error",
                        "{err}"
                    }
                }
                form {
                    action: "/register",
                    method: "post",
                    input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

                    label { class: "block text-xs font-bold text-gray-500 mb-1 uppercase",
                        "{t!(\"user.username\")}"
                    }
                    input {
                        r#type: "text",
                        class: "input-field",
                        name: "username",
                        value: "{username_value}",
                        required: true
                    }

                    label { class: "block text-xs font-bold text-gray-500 mb-1 uppercase",
                        "{t!(\"user.email\")}"
                    }
                    input {
                        r#type: "email",
                        class: "input-field",
                        name: "email",
                        value: "{email_value}",
                        required: true
                    }

                    label { class: "block text-xs font-bold text-gray-500 mb-1 uppercase",
                        "{t!(\"user.password\")}"
                    }
                    input {
                        r#type: "password",
                        class: "input-field",
                        name: "password",
                        required: true
                    }

                    label { class: "block text-xs font-bold text-gray-500 mb-1 uppercase",
                        "{t!(\"user.tribe\")}"
                    }
                    select {
                        class: "input-field",
                        name: "tribe",
                        option { value: "Roman", selected: selected_tribe == "Roman", "{t!(\"game.tribes.roman.title\")}" }
                        option { value: "Gaul", selected: selected_tribe == "Gaul", "{t!(\"game.tribes.gaul.title\")}" }
                        option { value: "Teuton", selected: selected_tribe == "Teuton", "{t!(\"game.tribes.teuton.title\")}" }
                    }


                    label { class: "block text-xs font-bold text-gray-500 mb-1 uppercase",
                      "{t!(\"game.starting_position.title\")}"
                    }
                    select {
                        class: "input-field",
                        name: "quadrant",
                        option { value: "NorthEast", selected: selected_quadrant == "NorthEast", "{t!(\"game.starting_position.north_east\", key=\"(+/+)\")}" }
                        option { value: "NorthWest", selected: selected_quadrant == "NorthWest", "{t!(\"game.starting_position.north_west\", key=\"(-/+)\")}" }
                        option { value: "SouthEast", selected: selected_quadrant == "SouthEast", "{t!(\"game.starting_position.south_east\", key=\"(+/-)\")}" }
                        option { value: "SouthWest", selected: selected_quadrant == "SouthWest", "{t!(\"game.starting_position.south_west\", key=\"(-/-)\")}" }
                    }

                    button {
                        r#type: "submit",
                        class: "btn-green mt-2",
                        "{t!(\"register.form.submit\")}"
                    }
                }
                div { class: "mt-6 text-center border-t border-gray-100 pt-4",
                    p { class: "text-sm text-gray-600",
                        "{t!(\"register.login_question\")}"
                    }
                    a {
                        href: "/login",
                        class: "text-green-700 font-bold hover:underline text-sm",
                        "{t!(\"register.login_button\")}"
                    }
                }
            }
        }
    }
}
