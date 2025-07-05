use floem::{
    IntoView,
    prelude::create_rw_signal,
    views::{Decorators, button, dyn_view},
    unit::DurationUnitExt,
};

pub fn launch() {
    floem::launch(app_view);
}

fn app_view() -> impl IntoView {
    let mut counter = create_rw_signal(0);
    (
        dyn_view(move || format!("Value: {}", counter)),
        (
            button("Increment")
                .action(move || counter += 1)
                .style(|s| s.border_radius(10))
                .animation(|a|
                    a.duration(1.seconds())
                        .keyframe(0, |f| f.computed_style())
                        .keyframe(50, |f| f.style(|s|
                            s.border_radius(0)))
                        .keyframe(100, |f| f.computed_style())
                        .auto_reverse(true)
                        .repeat(true)
                ),
            button("Decrement").action(move || counter -= 1),
        )
            .style(|s| s.flex_row().gap(6)),
    )
        .style(|s| s.flex_col().gap(6).items_center())
}
