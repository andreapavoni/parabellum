use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub username: String,
    pub village_count: i64,
    pub population: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaginationInfo {
    pub page: i64,
    pub per_page: i64,
    pub total_players: i64,
    pub total_pages: i64,
}

#[component]
pub fn StatsPage(entries: Vec<LeaderboardEntry>, pagination: PaginationInfo) -> Element {
    let has_prev = pagination.page > 1;
    let has_next = pagination.page < pagination.total_pages;

    let start_rank = if entries.is_empty() {
        0
    } else {
        ((pagination.page - 1) * pagination.per_page) + 1
    };
    let end_rank = if entries.is_empty() {
        0
    } else {
        start_rank + entries.len() as i64 - 1
    };

    rsx! {
        div { class: "max-w-2xl mx-auto space-y-4",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl mt-3 font-semibold text-gray-800", "Leaderboard" }
                div { class: "text-sm text-gray-600",
                    "Total players: {pagination.total_players}"
                }
            }

            div { class: "overflow-hidden border rounded-md bg-white shadow-sm",
                table { class: "min-w-full text-sm",
                    thead { class: "bg-gray-100 text-left text-gray-600 uppercase text-xs tracking-wide",
                        tr {
                            th { class: "px-4 py-3 w-16", "#" }
                            th { class: "px-4 py-3", "Player" }
                            th { class: "px-4 py-3 text-right", "Villages" }
                            th { class: "px-4 py-3 text-right", "Population" }
                        }
                    }
                    if entries.is_empty() {
                        tbody {
                            tr {
                                td { class: "px-4 py-4 text-center text-gray-500 text-sm", colspan: "4",
                                    "No players found yet."
                                }
                            }
                        }
                    } else {
                        tbody { class: "divide-y divide-gray-200",
                            for entry in entries {
                                tr { class: "hover:bg-gray-50",
                                    td { class: "px-4 py-3 font-mono text-gray-600", "{entry.rank}" }
                                    td { class: "px-4 py-3 font-semibold text-gray-800", "{entry.username}" }
                                    td { class: "px-4 py-3 text-right text-gray-700", "{entry.village_count}" }
                                    td { class: "px-4 py-3 text-right text-gray-900 font-semibold", "{entry.population}" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "flex items-center justify-between text-sm text-gray-600",
                div {
                    if pagination.total_players > 0 {
                        span { "Showing {start_rank}–{end_rank}" }
                    } else {
                        span { "No results to display" }
                    }
                }
                div { class: "flex items-center gap-2",
                    a {
                        href: if has_prev { format!("/stats?page={}", pagination.page - 1) } else { "#".to_string() },
                        class: if has_prev {
                            "px-3 py-1 rounded border border-gray-300 bg-white hover:bg-gray-50"
                        } else {
                            "px-3 py-1 rounded border border-gray-200 text-gray-400 cursor-not-allowed bg-gray-50"
                        },
                        "Prev"
                    }
                    span { class: "px-3 py-1 rounded bg-gray-100 border border-gray-200", "Page {pagination.page} / {pagination.total_pages}" }
                    a {
                        href: if has_next { format!("/stats?page={}", pagination.page + 1) } else { "#".to_string() },
                        class: if has_next {
                            "px-3 py-1 rounded border border-gray-300 bg-white hover:bg-gray-50"
                        } else {
                            "px-3 py-1 rounded border border-gray-200 text-gray-400 cursor-not-allowed bg-gray-50"
                        },
                        "Next"
                    }
                }
            }
        }
    }
}
