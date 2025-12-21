use dioxus::prelude::*;

/// Home page component - Landing page
#[component]
pub fn HomePage() -> Element {
    rsx! {
        // Navbar
        nav { class: "absolute w-full z-20 top-0 transition-all duration-300",
            div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                div { class: "flex justify-between items-center h-16 md:h-20",

                    // Logo
                    div { class: "flex-shrink-0 flex items-center",
                        a {
                            href: "/",
                            class: "font-serif font-bold text-xl md:text-2xl text-white tracking-wider hover:text-[#80ba34] transition shadow-sm",
                            "PARABELLUM"
                        }
                    }

                    // Desktop Menu
                    div { class: "hidden md:flex space-x-4 items-center",
                        a {
                            href: "/login",
                            class: "inline-flex items-center text-gray-100 hover:text-white font-medium px-3 py-2 transition text-sm uppercase tracking-wide",
                            "Login"
                        }
                        a {
                            href: "/register",
                            class: "inline-flex items-center bg-[#80ba34] hover:bg-[#6a9e2a] text-white font-bold px-5 py-2 rounded shadow transition text-sm uppercase tracking-wide",
                            "Register"
                        }
                    }

                    // Mobile Menu Button
                    div { class: "md:hidden flex items-center",
                        button {
                            "onclick": "document.getElementById('mobile-menu').classList.toggle('hidden')",
                            class: "text-gray-300 hover:text-white focus:outline-none p-1",
                            r#type: "button",
                            svg {
                                class: "h-5 w-5",
                                fill: "none",
                                view_box: "0 0 24 24",
                                stroke: "currentColor",
                                stroke_width: "2",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    d: "M4 6h16M4 12h16M4 18h16"
                                }
                            }
                        }
                    }
                }
            }

            // Mobile Menu Panel
            div {
                id: "mobile-menu",
                class: "hidden md:hidden bg-gray-900 border-b border-gray-800 absolute w-full top-16 left-0 px-4 pt-2 pb-4 shadow-xl",
                div { class: "flex flex-col space-y-3",
                    a {
                        href: "/login",
                        class: "block text-gray-300 hover:text-white hover:bg-gray-800 px-3 py-2 rounded-md text-base font-medium",
                        "Login"
                    }
                    a {
                        href: "/register",
                        class: "block bg-[#80ba34] text-white px-3 py-2 rounded-md text-base font-medium text-center",
                        "Register Now"
                    }
                }
            }
        }

        // Hero Section
        div { class: "relative bg-gray-900 overflow-hidden min-h-[600px] flex items-center",
            div { class: "absolute inset-0",
                img {
                    class: "w-full h-full object-cover",
                    src: "/static/header_landing.jpg",
                    alt: "Background"
                }
                div { class: "absolute inset-0 bg-gradient-to-b from-gray-900/90 via-gray-900/50 to-gray-900" }
            }

            div { class: "relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-20 md:py-32 flex flex-col items-center text-center",
                h1 { class: "text-4xl sm:text-5xl md:text-6xl lg:text-7xl font-extrabold tracking-tight text-white mb-6 leading-tight",
                    "Rule the "
                    br { class: "md:hidden" }
                    "Ancient World."
                }
                p { class: "mt-4 max-w-lg sm:max-w-2xl mx-auto text-lg sm:text-xl text-gray-300 leading-relaxed px-4",
                    "Develop your village, train armies, and fight for dominance. "
                    br { class: "hidden sm:block" }
                    "Pure strategy, open source, no pay-to-win."
                }

                div { class: "mt-10 flex flex-col sm:flex-row gap-4 w-full sm:w-auto px-4",
                    a {
                        href: "/register",
                        class: "w-full sm:w-auto px-8 py-4 bg-[#80ba34] hover:bg-[#6a9e2a] text-white text-lg font-bold rounded shadow-lg transition transform hover:scale-105 text-center",
                        "PLAY FOR FREE"
                    }
                    a {
                        href: "https://github.com/andreapavoni/parabellum",
                        target: "_blank",
                        class: "inline-flex items-center justify-center w-full sm:w-auto px-8 py-4 border border-gray-600 text-gray-300 hover:bg-gray-800 hover:text-white text-lg font-medium rounded transition",
                        svg {
                          class: "w-5 h-5",
                          fill: "currentColor",
                          view_box: "0 0 24 24",
                          path {
                            d: "M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"
                          }
                        }
                        "View Source"
                    }
                }
            }
        }

        section { class: "py-20 bg-gray-50 border-b border-gray-200 relative overflow-hidden",
            div { class: "pointer-events-none absolute -top-32 -left-24 h-72 w-72 rounded-full bg-gray-200/50 blur-3xl" }
            div { class: "pointer-events-none absolute -bottom-40 -right-24 h-80 w-80 rounded-full bg-gray-100/70 blur-3xl" }
            div { class: "relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",

                div { class: "text-center mb-16",
                    h2 { class: "text-4xl font-extrabold text-stone-900 sm:text-5xl tracking-tight",
                        "Choose Your Destiny"
                    }
                    div { class: "mx-auto mt-4 mb-6 h-1 w-24 rounded-full bg-gray-300" }
                    p { class: "text-lg text-stone-600 max-w-2xl mx-auto",
                        "Three unique tribes, three distinct playstyles. Will you choose discipline, speed, or brute force?"
                    }
                }

                div { class: "grid grid-cols-1 md:grid-cols-3 gap-8 lg:gap-10",
                    // ROMANS
                    div { class: "bg-white rounded-lg shadow-lg overflow-hidden border border-gray-200 transition duration-300 transform hover:-translate-y-1 hover:shadow-xl flex flex-col",
                        div { class: "h-1.5 w-full bg-gradient-to-r from-red-700 via-red-500 to-amber-400" }
                        div { class: "p-8 flex-grow",
                            h3 { class: "text-2xl font-bold text-stone-900 mb-3", "The Romans" }
                            p { class: "text-stone-600 text-sm leading-relaxed mb-6",
                                "Masters of engineering and discipline. Their troops are elite but expensive. The only tribe capable of simultaneous construction."
                            }
                            ul { class: "space-y-2.5 mb-6",
                                li { class: "flex items-center text-xs font-semibold text-red-700 uppercase tracking-wider",
                                    span { class: "w-2.5 h-2.5 rounded-full bg-red-600 mr-2 ring-2 ring-red-200/70 shadow-[0_0_6px_rgba(185,28,28,0.35)]" }
                                    "High Defense"
                                }
                                li { class: "flex items-center text-xs font-semibold text-red-700 uppercase tracking-wider",
                                    span { class: "w-2.5 h-2.5 rounded-full bg-red-600 mr-2 ring-2 ring-red-200/70 shadow-[0_0_6px_rgba(185,28,28,0.35)]" }
                                    "Elite Infantry"
                                }
                            }
                        }
                    }

                    // GAULS
                    div { class: "bg-white rounded-lg shadow-lg overflow-hidden border border-gray-200 transition duration-300 transform hover:-translate-y-1 hover:shadow-xl flex flex-col",
                        div { class: "h-1.5 w-full bg-gradient-to-r from-blue-800 via-blue-600 to-teal-400" }
                        div { class: "p-8 flex-grow",
                            h3 { class: "text-2xl font-bold text-stone-900 mb-3", "The Gauls" }
                            p { class: "text-stone-600 text-sm leading-relaxed mb-6",
                                "Swift and defensive. Known for the fastest cavalry and unique traps to protect their villages. The best choice for tactical players."
                            }
                            ul { class: "space-y-2.5 mb-6",
                                li { class: "flex items-center text-xs font-semibold text-blue-700 uppercase tracking-wider",
                                    span { class: "w-2.5 h-2.5 rounded-full bg-blue-600 mr-2 ring-2 ring-blue-200/70 shadow-[0_0_6px_rgba(29,78,216,0.35)]" }
                                    "Fastest Speed"
                                }
                                li { class: "flex items-center text-xs font-semibold text-blue-700 uppercase tracking-wider",
                                    span { class: "w-2.5 h-2.5 rounded-full bg-blue-600 mr-2 ring-2 ring-blue-200/70 shadow-[0_0_6px_rgba(29,78,216,0.35)]" }
                                    "Trap Defense"
                                }
                            }
                        }
                    }

                    // TEUTONS
                    div { class: "bg-white rounded-lg shadow-lg overflow-hidden border border-gray-200 transition duration-300 transform hover:-translate-y-1 hover:shadow-xl flex flex-col",
                        div { class: "h-1.5 w-full bg-gradient-to-r from-amber-800 via-amber-600 to-yellow-400" }
                        div { class: "p-8 flex-grow",
                            h3 { class: "text-2xl font-bold text-stone-900 mb-3", "The Teutons" }
                            p { class: "text-stone-600 text-sm leading-relaxed mb-6",
                                "Fearless raiders. Their troops are cheap and quick to train, overwhelming enemies with numbers. Perfect for aggressive playstyles."
                            }
                            ul { class: "space-y-2.5 mb-6",
                                li { class: "flex items-center text-xs font-semibold text-amber-800 uppercase tracking-wider",
                                    span { class: "w-2.5 h-2.5 rounded-full bg-amber-700 mr-2 ring-2 ring-amber-200/70 shadow-[0_0_6px_rgba(180,83,9,0.35)]" }
                                    "Cheap Army"
                                }
                                li { class: "flex items-center text-xs font-semibold text-amber-800 uppercase tracking-wider",
                                    span { class: "w-2.5 h-2.5 rounded-full bg-amber-700 mr-2 ring-2 ring-amber-200/70 shadow-[0_0_6px_rgba(180,83,9,0.35)]" }
                                    "Raid Bonus"
                                }
                            }
                        }
                    }

                }
            }
        }

        // Feature 1: Resources
        section { class: "py-16 md:py-24 bg-white",
            div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                div { class: "flex flex-col lg:grid lg:grid-cols-2 lg:gap-16 items-center gap-10",

                    // Text
                    div { class: "text-center lg:text-left",
                        h2 { class: "text-3xl font-extrabold text-gray-900 sm:text-4xl mb-4",
                            "Build Your Empire"
                        }
                        p { class: "text-lg text-gray-600 mb-6",
                            "Start with a humble village. Manage resources, construct buildings, and optimize your economy to fuel your expansion."
                        }
                        ul { class: "space-y-3 inline-block text-left",
                            li { class: "flex items-center text-gray-700",
                                span { class: "h-6 w-6 rounded-full bg-green-100 text-green-600 flex items-center justify-center mr-3 text-xs flex-shrink-0",
                                    "✓"
                                }
                                "Resource field mechanics"
                            }
                            li { class: "flex items-center text-gray-700",
                                span { class: "h-6 w-6 rounded-full bg-green-100 text-green-600 flex items-center justify-center mr-3 text-xs flex-shrink-0",
                                    "✓"
                                }
                                "Construct and upgrade"
                            }
                            li { class: "flex items-center text-gray-700",
                                span { class: "h-6 w-6 rounded-full bg-green-100 text-green-600 flex items-center justify-center mr-3 text-xs flex-shrink-0",
                                    "✓"
                                }
                                "Research technologies"
                            }
                        }
                    }

                    // Image
                    div { class: "w-full browser-frame transform hover:scale-[1.01] transition duration-500 shadow-xl",
                        div { class: "browser-header",
                            div { class: "dot" }
                            div { class: "dot" }
                            div { class: "dot" }
                        }
                        img {
                            src: "/static/screenshots_resources.png",
                            alt: "Village View",
                            class: "w-full h-auto object-cover aspect-[4/3]"
                        }
                    }
                }
            }
        }

        // Feature 2: Village
        section { class: "py-16 md:py-24 bg-gray-50",
            div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                div { class: "flex flex-col lg:grid lg:grid-cols-2 lg:gap-16 items-center gap-10",

                    // Image (first on desktop, second on mobile)
                    div { class: "w-full order-2 lg:order-1 browser-frame transform hover:scale-[1.01] transition duration-500 shadow-xl",
                        div { class: "browser-header",
                            div { class: "dot" }
                            div { class: "dot" }
                            div { class: "dot" }
                        }
                        img {
                            src: "/static/screenshots_village.png",
                            alt: "Map View",
                            class: "w-full h-auto object-cover aspect-[4/3]"
                        }
                    }

                    // Text
                    div { class: "order-1 lg:order-2 text-center lg:text-left",
                        h2 { class: "text-3xl font-extrabold text-gray-900 sm:text-4xl mb-4",
                            "Strategic Infrastructure"
                        }
                        p { class: "text-lg text-gray-600 mb-6",
                            "Inside the walls lies the heart of your civilization. Build military academies to train massive armies, marketplaces to trade with neighbors, and embassies to forge powerful alliances."
                        }
                        ul { class: "space-y-3 inline-block text-left",
                            li { class: "flex items-center text-gray-700",
                                span { class: "h-6 w-6 rounded-full bg-blue-100 text-blue-600 flex items-center justify-center mr-3 text-xs flex-shrink-0",
                                    "✓"
                                }
                                "Train Infantry & Cavalry"
                            }
                            li { class: "flex items-center text-gray-700",
                                span { class: "h-6 w-6 rounded-full bg-blue-100 text-blue-600 flex items-center justify-center mr-3 text-xs flex-shrink-0",
                                    "✓"
                                }
                                "Global Marketplace Trading"
                            }
                            li { class: "flex items-center text-gray-700",
                                span { class: "h-6 w-6 rounded-full bg-blue-100 text-blue-600 flex items-center justify-center mr-3 text-xs flex-shrink-0",
                                    "✓"
                                }
                                "Unlock Advanced Tech Trees"
                            }
                        }
                    }
                }
            }
        }

        // Feature 3: Map
        section { class: "py-16 md:py-24 bg-white",
            div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                div { class: "flex flex-col lg:grid lg:grid-cols-2 lg:gap-16 items-center gap-10",
                    // Text
                    div { class: "text-center lg:text-left",
                        h2 { class: "text-3xl font-extrabold text-gray-900 sm:text-4xl mb-4",
                            "Explore & Conquer"
                        }
                        p { class: "text-lg text-gray-600 mb-6",
                            "The world is vast and dangerous. Scout neighbors, form alliances, and launch coordinated attacks to control your sector."
                        }
                        ul { class: "space-y-3 inline-block text-left",
                            li { class: "flex items-center text-gray-700",
                                span { class: "h-6 w-6 rounded-full bg-blue-100 text-blue-600 flex items-center justify-center mr-3 text-xs flex-shrink-0",
                                    "✓"
                                }
                                "Infinite persistent map"
                            }
                            li { class: "flex items-center text-gray-700",
                                span { class: "h-6 w-6 rounded-full bg-blue-100 text-blue-600 flex items-center justify-center mr-3 text-xs flex-shrink-0",
                                    "✓"
                                }
                                "Real-time movement"
                            }
                            li { class: "flex items-center text-gray-700",
                                span { class: "h-6 w-6 rounded-full bg-blue-100 text-blue-600 flex items-center justify-center mr-3 text-xs flex-shrink-0",
                                    "✓"
                                }
                                "Raid, siege, reinforce"
                            }
                        }
                    }
                    // Image
                    div { class: "w-full browser-frame transform hover:scale-[1.01] transition duration-500 shadow-xl",
                        div { class: "browser-header",
                            div { class: "dot" }
                            div { class: "dot" }
                            div { class: "dot" }
                        }
                        img {
                            src: "/static/screenshots_map.png",
                            alt: "Map View",
                            class: "w-full h-auto object-cover aspect-[4/3]"
                        }
                    }
                }
            }
        }

        // CTA Section
        section { class: "bg-[#6a9e2a] py-16",
            div { class: "max-w-4xl mx-auto px-4 text-center",
                h2 { class: "text-2xl md:text-3xl font-bold text-white mb-4",
                    "Ready to make history?"
                }
                p { class: "text-green-100 mb-8 text-base md:text-lg",
                    "Join the alpha testing today."
                }
                a {
                    href: "/register",
                    class: "inline-block bg-white text-[#6a9e2a] font-bold text-lg md:text-xl px-10 py-4 rounded shadow-lg hover:bg-gray-100 transition transform hover:-translate-y-1 w-full sm:w-auto",
                    "Start Now"
                }
            }
        }

        // Footer
        footer { class: "bg-gray-900 text-gray-400 py-10 border-t border-gray-800",
            div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 flex flex-col md:flex-row justify-between items-center text-center md:text-left",
              p {
                  "A "
                  a { class: "hover:underline", href: "https://pavonz.com", "pavonz" }
                  " joint © 2025 | "
                  a {
                      class: "hover:underline",
                      href: "https://github.com/andreapavoni/parabellum",
                      "Github"
                  }
              }
              div { class: "mt-2 space-x-3",
                  span { "{t!(\"footer.not_affiliated\", name = \"Travian Games GmbH\")}" }
              }
            }
        }
    }
}
