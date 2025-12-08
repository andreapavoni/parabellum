use dioxus::prelude::*;
use rust_i18n::t;

/// Home page component
#[component]
pub fn HomePage() -> Element {
    rsx! {
      div { class: "grid md:grid-cols-2 gap-8 items-start",
          div { class: "space-y-6",
              div { class: "content-box p-6 bg-white",
                  h1 { class: "text-2xl font-bold text-gray-700 mb-4", "{t!(\"home.welcome\")}" }
                  p { class: "text-sm leading-relaxed text-gray-600 mb-4",
                      "{t!(\"home.description\")}"
                  }
                  div { class: "mt-6 text-center",
                      a {
                          class: "btn-green px-8 py-3 rounded text-lg inline-block shadow-md transform hover:-translate-y-1 transition",
                          href: "travian_register.html",
                          "{t!(\"home.play_for_free\")}"
                      }
                      p { class: "text-xs text-gray-400 mt-2", "{t!(\"home.no_downloads\")}" }
                  }
              }
              div { class: "content-box p-4",
                  h3 { class: "font-bold text-sm text-gray-500 border-b border-gray-200 pb-2 mb-3",
                      "{t!(\"home.latest_news\")}"
                  }
                  div { class: "space-y-3 text-xs",
                      div { class: "flex justify-between",
                          span { class: "font-bold text-green-700", "{t!(\"home.new_speed_server3x\")}" }
                          span { class: "text-gray-400", "{t!(\"home.today\")}" }
                      }
                  }
              }
          }
          div { class: "hidden md:block relative h-80 rounded-lg overflow-hidden border border-gray-300 shadow-md group",
              div {
                  class: "absolute inset-0 bg-cover bg-center transition duration-1000 group-hover:scale-105",
                  style: "background-image: url('https://wallpaperaccess.com/full/1736218.jpg');",
              }
              div { class: "absolute inset-0 bg-gradient-to-t from-black/60 to-transparent flex items-end p-6",
                  div { class: "text-white",
                      h3 { class: "font-bold text-xl text-shadow", "{t!(\"home.living_world\")}" }
                      p { class: "text-sm text-gray-200", "{t!(\"home.many_players\")}" }
                  }
              }
          }
      }
      div { class: "mt-8 grid grid-cols-3 gap-4 text-center text-sm text-gray-600",
          div { class: "content-box p-3",
              div { class: "font-bold text-lg text-green-600", "12,405" }
              div { class: "text-xs uppercase tracking-wide text-gray-400", "{t!(\"home.players\")}" }
          }
          div { class: "content-box p-3",
              div { class: "font-bold text-lg text-orange-600", "4,200" }
              div { class: "text-xs uppercase tracking-wide text-gray-400",
                  "{t!(\"home.active_today\")}"
              }
          }
          div { class: "content-box p-3",
              div { class: "font-bold text-lg text-blue-600", "Dec 25th" }
              div { class: "text-xs uppercase tracking-wide text-gray-400",
                  "{t!(\"home.server_starting\")}"
              }
          }
      }
    }
}

/// Login page component
#[component]
pub fn LoginPage(csrf_token: String, email_value: String, error: Option<String>) -> Element {
    rsx! {
        div { class: "flex flex-col items-center w-full px-4",
            div { class: "login-box",
                h2 { class: "text-xl font-bold text-gray-700 mb-6 text-center",
                    "{t!(\"login.form.title\")}"
                }
                if let Some(err) = error {
                    p { class: "mb-3 text-center text-red-600 error",
                        "{err}"
                    }
                }
                form {
                    action: "/login",
                    method: "post",
                    input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }

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

                    div { class: "flex justify-between items-center mb-1",
                        label { class: "block text-xs font-bold text-gray-500 uppercase",
                            "{t!(\"user.password\")}"
                        }
                    }
                    input {
                        r#type: "password",
                        class: "input-field",
                        name: "password",
                        required: true
                    }

                    button {
                        r#type: "submit",
                        class: "btn-green mt-2",
                        "{t!(\"login.form.submit\")}"
                    }
                }
                div { class: "mt-6 text-center border-t border-gray-100 pt-4",
                    p { class: "text-sm text-gray-600",
                        "{t!(\"login.register_question\")}"
                    }
                    a {
                        href: "/register",
                        class: "text-green-700 font-bold hover:underline text-sm",
                        "{t!(\"login.register_button\")}"
                    }
                }
            }
        }
    }
}

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
                        name: selected_quadrant.clone(),
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
