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
                          href: "/register",
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
                  style: "background-image: url('/static/1736218.jpg');",
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
