use floem::{
    peniko::Color, prelude::{create_rw_signal, RwSignal, SignalGet, SignalUpdate}, style::FontStyle, taffy::{AlignItems, FlexDirection, JustifyContent}, views::{
        button, dyn_stack, dyn_view, text_input, Decorators, DynStack, DynamicView
    }, IntoView
};

pub fn launch() {
    floem::launch(app_view);
}

fn app_view() -> impl IntoView {
    let new_todo = create_rw_signal(String::new());
    let todo_list = TodoList::new(vec![
        Todo::new("Learn Rust", false),
        Todo::new("Build a web app", true),
    ]);

    let add_todo = move || {
        let new_todo_value = new_todo.get();
        if !new_todo_value.is_empty() {
            new_todo.set(String::new());
            todo_list.update(|list| {
                println!("Adding new todo: {}", new_todo_value);
                list.todos.push(Todo::new(&new_todo_value, false));
            });
        }
    };

    (
        dyn_view(move || todo_list.get()),
        (text_input(new_todo), button("Add Todo").action(add_todo)).style(|s| {
            s.align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween)
                .gap(8)
        }),
    )
        .style(|s| s.padding(16).flex_direction(FlexDirection::Column).gap(8))
}

#[derive(Clone)]
struct Todo {
    title: String,
    completed: RwSignal<bool>,
    removed: RwSignal<bool>,
}

impl Todo {
    fn new(title: &str, completed: bool) -> Self {
        Self {
            title: title.to_string(),
            completed: create_rw_signal(completed),
            removed: create_rw_signal(false),
        }
    }
}

impl IntoView for Todo {
    type V = DynamicView;

    fn into_view(self) -> Self::V {
        dyn_view(move || {
            let completed = self.completed.get();
            (
                button(if completed { "✓" } else { "✗" }.style(|s| {
                    s.width(20)
                        .height(20)
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center)
                }))
                .action(move || {
                    self.completed.set(!self.completed);
                }),
                self.title.clone(),
                button("Remove")
                    .action(move || {
                        self.removed.set(true);
                    })
                    .style(|s| {
                        s.padding(4)
                            .border_radius(4)
                            .background(Color::from_rgb8(255, 0, 0))
                    }),
            )
                .style(move |s| s.align_items(AlignItems::Center).gap(4))
        })
    }
}

#[derive(Clone)]
struct TodoList {
    todos: Vec<Todo>,
}

impl TodoList {
    fn new(todos: Vec<Todo>) -> RwSignal<Self> {
        create_rw_signal(Self { todos })
    }
}

impl IntoView for TodoList {
    type V = DynStack<Todo>;

    fn into_view(self) -> Self::V {
        dyn_stack(
            move || {
                self.todos
                    .clone()
                    .into_iter()
                    .filter(|todo| !todo.removed.get())
            },
            |todo| todo.title.clone(),
            |todo| todo.clone(),
        )
        .style(|s| s.flex_direction(FlexDirection::Column).gap(8))
    }
}
