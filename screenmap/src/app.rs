use crate::{interface::DataCell, server::{get_rows, get_screen_keys, search_table, search_tbls}};
use futures::FutureExt;
use leptos_use::{on_click_outside, use_resize_observer};
use leptos::{
    html::Div, logging::{error, log}, prelude::*, reactive::signal::signal
};
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/screenmap.css" />
        <Title text="Welcome to Leptos" />
        <Router>
            <main class="app-container">
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=SearchTables />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn SearchTables() -> impl IntoView {
    let (screen_name_0, set_screen_name_0) = signal(String::new());
    let (screen_name_1, set_screen_name_1) = signal(String::new());
    let (screen_name_2, set_screen_name_2) = signal(String::new());
    let (show_screen_0, set_show_screen_0) = signal(false);
    let (show_screen_1, set_show_screen_1) = signal(false);
    let (show_screen_2, set_show_screen_2) = signal(false);
    let (to_set, set_to_set) = signal(0usize);

    let (search_query, set_search_query) = signal("".to_string());
    let (query, set_query) = signal(None);
    let (is_search_focused, set_is_search_focused) = signal(false);
    let (page_size, set_page_size) = signal(10usize);
    let table_page_size = Signal::derive(move || {
        let num_tables = show_screen_0() as usize
            + show_screen_1() as usize
            + show_screen_2() as usize;
        page_size().checked_div(num_tables).unwrap_or(0)
    });

    let search_container_ref = NodeRef::<Div>::new();
    let table_viewport_ref = NodeRef::<Div>::new();

    let _ = on_click_outside(
        search_container_ref,
        move |_| set_is_search_focused(false),
    );
    let _ = use_resize_observer(
        table_viewport_ref,
        move |entries, _| {
            if let Some(entry) = entries.first() {
                let height = entry.content_rect().height();
                let new_page_size = (height / 40.0).floor() as usize;
                set_page_size.set(new_page_size.max(1)); // Ensure at least 1 row
            }
        }
    );

    let matches = LocalResource::new(move || search_tbls(search_query()));
    let search_matches = move || {
        matches
            .get()
            .unwrap_or(Ok(vec![]))
            .unwrap_or_else(|e| {
                error!("Search matches errored: {e}.");
                vec![]
            })
            .into_iter()
            .map(|item| {
                view! {
                    <div
                        on:click=move |_| { 
                            let (set_name, set_show) = match to_set() {
                                0 => (set_screen_name_0, set_show_screen_0),
                                1 => (set_screen_name_1, set_show_screen_1),
                                2 => (set_screen_name_2, set_show_screen_2),
                                _ => return,
                            };
                            set_to_set.set((to_set() + 1) % 3);
                            set_name(item.clone());
                            set_show(true);
                            set_is_search_focused(false);
                        }
                        style="cursor: pointer; padding: 5px;"
                    >
                        {item.clone()}
                    </div>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <div class="search-tables-container">
            <div class="search-header">
                <div class="table-search-wrapper">
                    <input
                        type="text"
                        placeholder="Search tables..."
                        class="generic-box"
                        prop:value=search_query
                        on:click=move |_| set_is_search_focused(true)
                        on:input=move |ev| {
                            let text = event_target_value(&ev);
                            set_search_query(text);
                        }
                    />
                    <Show when=is_search_focused>
                        <div class="search-results" node_ref=search_container_ref>
                            <Transition fallback=move || view! { <p>"Loading..."</p> }>
                                {search_matches}
                            </Transition>
                        </div>
                    </Show>
                </div>
                <div class="logo-container">
                    <img src="/logo.webp" alt="Logo" class="logo"/>
                </div>
                <div class="in-table-search">
                    <input
                        type="text"
                        class="generic-box"
                        placeholder="Search in table..."
                        prop:value=query
                        on:input=move |ev| {
                            let text = event_target_value(&ev);
                            set_query(Some(text));
                        }
                    />
                </div>
            </div>

            <div class="tables-viewport" node_ref=table_viewport_ref>
                <div class="table-viewport">
                    <Show when=move || show_screen_0.get() fallback=|| view! {}.into_view()>
                        <Table screen_name=screen_name_0 query page_size=table_page_size/>
                    </Show>
                </div>
                <div class="table-viewport">
                    <Show when=move || show_screen_1.get() fallback=|| view! {}.into_view()>
                        <Table screen_name=screen_name_1 query page_size=table_page_size/>
                    </Show>
                </div>
                <div class="table-viewport">
                    <Show when=move || show_screen_2.get() fallback=|| view! {}.into_view()>
                        <Table screen_name=screen_name_2 query page_size=table_page_size/>
                    </Show>
                </div>
            </div>
        </div>
    }
}

#[component]
fn Table(
    screen_name: ReadSignal<String>,
    query: ReadSignal<Option<String>>,
    page_size: Signal<usize>,
) -> impl IntoView {
    let screen_keys = Resource::new(
        move || screen_name.get(),
        |screen_name| get_screen_keys(screen_name).map(|result| result.unwrap_or_default())
    );
    let (cur_page, set_cur_page) = signal(0usize);
    Effect::new(move || {
        let _ = query();
        let _ = page_size();
        let _ = screen_name();
        set_cur_page(0);
    });
    let rows_getter = Resource::new(
        move || {
            (
                query.get(),
                cur_page.get(),
                screen_name.get(),
                page_size.get(),
            )
        },
        move |(query, cur_page, screen_name, page_size)| async move {
            let query = query.unwrap_or_default();
            let start = cur_page * page_size + 1;
            let end = start + page_size;
            let mut rows_to_view = vec![];
            let ranges = search_table(screen_name.clone(), query)
                .await
                .map_err(|e| e.to_string())?;
            let mut total_rows = 0usize;
            for range in ranges.iter() {
                let prev_total = total_rows;
                total_rows += *range.end() - *range.start() + 1;
                if total_rows < start {
                    continue;
                } else if prev_total < end {
                    let cur_start = range.start() + start.saturating_sub(prev_total);
                    let cur_end = range.end() - total_rows.saturating_sub(end);
                    rows_to_view.extend(cur_start..=cur_end);
                }
            }
            let fetched_rows = get_rows(rows_to_view.clone(), screen_name)
                    .await
                    .map_err(|e| e.to_string())?
            ;
            let mut fetched_rows_iter = fetched_rows.iter();
            let mut cur_fetched = fetched_rows_iter.next();
            let mut viewable_rows = vec![];
            for i in rows_to_view {
                if let Some((row_id, row)) = cur_fetched
                    && *row_id == i
                {
                    viewable_rows.push(row.clone());
                    cur_fetched = fetched_rows_iter.next();
                } else {
                    return Err(format!(
                        "row not in fetched_rows_iter {} {:?} {:?}",
                        i, viewable_rows, cur_fetched
                    ));
                }
            }
            Ok((fetched_rows, viewable_rows, total_rows))
        },
    );

    let (num_rows, set_num_rows) = signal(0);
    Effect::new(move || {
        set_num_rows(rows_getter.get().map_or(0, |k| k.map_or(0, |(_, _, total)| total)))
    });

    let table_header = move || match screen_keys.get() {
        Some(screen_keys) => {
            let header_inner = screen_keys
                .into_iter()
                .map(|(key, _, _)| view! { <th>{key}</th> })
                .collect_view();
            view! {
                <thead>
                    <tr>{header_inner}</tr>
                </thead>
            }
            .into_any()
        }
        None => view! {
            <thead>
                <tr></tr>
            </thead>
        }
        .into_any(),
    };

    let display_cell = |(col, bound): (&DataCell, Option<(f64, f64)>)| {
        let style = if let DataCell::Double(x) = col {
            if let Some((min, max)) = bound {
                let fraction = if *x >= 0.0 {
                    if max > 0.0 { (*x / max).clamp(0.0, 1.0) } else { 0.0 }
                } else {
                    if min < 0.0 { (*x / min).clamp(0.0, 1.0) } else { 0.0 }
                };
                if *x >= 0.0 {
                    let intensity = (fraction * 128.0).round() as u8;
                    format!("background-color: rgb({},{},{});", 
                        255 - intensity,
                        255 - intensity,
                        255)
                } else {
                    // Interpolate white to green for negative values
                    let intensity = (fraction * 255.0).round() as u8;
                    format!("background-color: rgb({},{},{});", 
                        255 - intensity, 
                        255, 
                        255 - intensity)
                }
            } else {
                "background-color: white;".to_string()
            }
        } else {
            "background-color: white;".to_string()
        };
        let cell_data = match col {
            DataCell::Double(x) => view! { {*x} }.into_any(),
            DataCell::Null => view! {}.into_any(),
            DataCell::BigInt(n) => view! { {*n} }.into_any(),
            DataCell::Text(s) => view! { {s.clone()} }.into_any(),
        };
        view! { <td class="cell-border" style=style>{cell_data}</td> }
    };

    let table_body = move || {
        match rows_getter.get() {
            Some(Ok((_, items, _))) => {
                let rows = items.into_iter().map(|item| {
                    screen_keys
                        .get()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(key, _, bound)| item.get(&key).map(move |item| (item, bound)))
                        .try_collect::<Vec<_>>()
                        .map(|row| view! {
                            <tr> {
                                row.into_iter().map(display_cell).collect_view()
                            } </tr> 
                        })
                }).collect_view();

                view! { <tbody>{rows}</tbody> }.into_any()
            }
            Some(Err(e)) => view! {
                <tbody>
                    <tr>
                        <td class="cell-border" colspan=screen_keys.get().map(|k| k.len()).unwrap_or(1)>
                            {e}
                        </td>
                    </tr>
                </tbody>
            }.into_any(),
            None => view! {
                <tbody>
                    <tr>
                        <td class="cell-border" colspan=screen_keys.get().map(|k| k.len()).unwrap_or(1)>
                            "Loading..."
                        </td>
                    </tr>
                </tbody>
            }.into_any(),
        }
    };

    view! {
        <div class="outer-container">
            <Transition>
                <div class="table-controls">
                    <button
                        on:click=move |_| set_cur_page.update(|p| *p = p.saturating_sub(1))
                        disabled=move || cur_page.get() == 0
                        class="generic-box"
                    >
                        "Previous"
                    </button>
                    <span class="page-indicator">{ move ||
                        format!(
                            "Showing {} - {} of {} ({})",
                            cur_page.get() * page_size.get() + 1,
                            std::cmp::min((cur_page.get() + 1) * page_size.get(), num_rows.get()),
                            num_rows.get(),
                            screen_name.get()
                        )
                    }</span>
                    <button
                        on:click=move |_| set_cur_page.update(|p| *p += 1)
                        disabled=move || { (cur_page() + 1) * page_size.get() >= num_rows.get() }
                        class="generic-box"
                    >
                        "Next"
                    </button>
                </div>
                <div class="scroll-container">
                    <ErrorBoundary fallback=|errors| {
                        view! {
                            <div class="error">
                                {errors
                                    .get()
                                    .into_iter()
                                    .map(|(_, error)| view! { <p>{error.to_string()}</p> })
                                    .collect_view()}
                            </div>
                        }
                    }>
                        <table class="bordered-table">
                            {move || table_header()}
                            {move || table_body()}
                        </table>
                    </ErrorBoundary>
                </div>
            </Transition>
        </div>
    }
}
