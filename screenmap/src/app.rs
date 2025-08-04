use crate::server::{get_rows, get_screen_keys, search_table, search_tbls};
use leptos_use::{on_click_outside, use_resize_observer};
use leptos::{
    html::Div, logging::{error, log}, prelude::*, reactive::signal::signal
};
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};
use std::collections::BTreeMap;

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
    let (screen_name, set_screen_name) = signal(None);
    let (search_query, set_search_query) = signal("".to_string());
    let (query, set_query) = signal(None);
    let (is_search_focused, set_is_search_focused) = signal(false);
    let (page_size, set_page_size) = signal(10);

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
                log!("h: {}", height);
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
                            set_screen_name(Some(item.clone()));
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

            <div class="table-viewport" node_ref=table_viewport_ref>
                <Show when=move || screen_name().is_some() fallback=|| view! {}.into_view()>
                    <Table screen_name query page_size/>
                </Show>
            </div>
        </div>
    }
}

#[component]
fn Table(
    screen_name: ReadSignal<Option<String>>,
    query: ReadSignal<Option<String>>,
    page_size: ReadSignal<usize>,
) -> impl IntoView {
    let screen_keys = Resource::new(
        move || screen_name,
        async |screen_name| {
            if let Some(name) = screen_name.get() {
                get_screen_keys(name).await.unwrap_or(vec![])
            } else {
                vec![]
            }
        },
    );
    let (cur_page, set_cur_page) = signal(0usize);
    Effect::new(move || {
        let _ = query.get();
        set_cur_page(0);
    });
    let known_rows = RwSignal::new(BTreeMap::<_, BTreeMap<String, String>>::new());
    let rows_getter = Resource::new(
        move || {
            (
                query.get(),
                cur_page.get(),
                screen_name.get(),
                known_rows.get(),
                page_size.get(),
            )
        },
        move |(query, cur_page, screen_name, known_rows, page_size)| async move {
            let query = query.unwrap_or_default();
            let screen_name = screen_name.ok_or(String::from("No screen name."))?;
            let start = cur_page * page_size + 1;
            let end = start + page_size;
            let mut rows_to_view = vec![];
            let ranges = search_table(screen_name.clone(), query)
                .await
                .map_err(|e| e.to_string())?;
            let mut total_rows = 1usize;
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
            let to_fetch: Vec<_> = rows_to_view
                .iter()
                .filter_map(|i| (!known_rows.contains_key(i)).then_some(*i))
                .collect();
            let fetched_rows = if to_fetch.is_empty() {
                vec![]
            } else {
                get_rows(to_fetch, screen_name)
                    .await
                    .map_err(|e| e.to_string())?
            };
            let mut fetched_rows_iter = fetched_rows.iter();
            let mut cur_fetched = fetched_rows_iter.next();
            let mut viewable_rows = vec![];
            for i in rows_to_view {
                if let Some((row_id, row)) = cur_fetched
                    && *row_id == i
                {
                    viewable_rows.push(row.clone());
                    cur_fetched = fetched_rows_iter.next();
                } else if let Some(row) = known_rows.get(&i) {
                    viewable_rows.push(row.clone())
                } else {
                    return Err(format!(
                        "row not in fetched_rows_iter or known_rows {} {:?} {:?} {:?}",
                        i, viewable_rows, cur_fetched, known_rows
                    ));
                }
            }
            Ok((fetched_rows, viewable_rows, total_rows))
        },
    );
    Effect::new(move || {
        if let Some(Ok((fetched_rows, _, _))) = rows_getter.get() {
            known_rows.update(|known_rows_mut| {
                known_rows_mut.extend(fetched_rows);
            });
        }
    });

    let (num_rows, set_num_rows) = signal(0);
    Effect::new(move || {
        set_num_rows(rows_getter.get().map_or(0, |k| k.map_or(0, |(_, _, total)| total)))
    });

    let table_header = move || match screen_keys.get() {
        Some(screen_keys) => {
            let header_inner = screen_keys
                .into_iter()
                .map(|(key, _)| view! { <th>{key}</th> })
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

    let table_body = move || {
        match rows_getter.get() {
            Some(Ok((_, items, _))) => {
                let rows = items.into_iter().map(|item| {
                    screen_keys
                        .get()
                        .unwrap_or_default()
                        .iter()
                        .map(|(key, _)| item.get(key))
                        .try_collect::<Vec<_>>()
                        .map(|row| view! { 
                            <tr>
                                {row.into_iter().map(|col| view! {
                                    <td class="cell-border">{col.clone()}</td> 
                                }).collect_view()}
                            </tr> 
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
                            "Showing {} - {} of {}",
                            cur_page.get() * page_size.get() + 1,
                            std::cmp::min((cur_page.get() + 1) * page_size.get(), num_rows.get()),
                            num_rows.get()
                        )
                    }</span>
                    <button
                        on:click=move |_| set_cur_page.update(|p| *p += 1)
                        disabled=move || { (cur_page.get() + 1) * page_size.get() >= num_rows.get() }
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
