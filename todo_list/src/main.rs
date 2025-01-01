use yew::prelude::*;
use yew::Renderer;
use todo_list::Task;

fn main() {
    Renderer::<App>::new().render();
}

#[function_component(App)]
fn app() -> Html {
    let tasks = use_state(|| Vec::<Task>::new());
    let next_id = use_state(|| 1);

    let add_task = {
        let tasks = tasks.clone();
        let next_id = next_id.clone();

        Callback::from(move |description: String| {
            let mut new_tasks = (*tasks).clone();

            new_tasks.push(Task {
                id: *next_id,
                description,
                completed: false,
            });
            tasks.set(new_tasks);
            next_id.set(*next_id + 1);
        })
    };

    let toggle_task = {
        let tasks = tasks.clone();

        Callback::from(move |id: u32| {
            let mut new_tasks = (*tasks).clone();

            if let Some(task) = new_tasks.iter_mut().find(|t| t.id == id) {
                task.completed = !task.completed;
            }
            tasks.set(new_tasks);
        })
    };

    let delete_task = {
        let tasks = tasks.clone();
        Callback::from(move |id: u32| {
            let new_tasks: Vec<Task> = tasks.iter().cloned().filter(|t| t.id != id).collect();
            tasks.set(new_tasks);
        })
    };

    let edit_task = {
        let tasks = tasks.clone();

        Callback::from(move |(id, new_description): (u32, String)| {
            let mut new_tasks = (*tasks).clone();
            if let Some(task) = new_tasks.iter_mut().find(|t| t.id == id) {
                task.description = new_description;
            }
            tasks.set(new_tasks);
        })
    };

    html! {
        <div class="todo-app">
            <h1>{ "To-Do List" }</h1>
            <AddTask on_add={add_task} />
            <TaskList
                tasks={(*tasks).clone()}
                on_toggle={toggle_task}
                on_edit={edit_task}
                on_delete={delete_task}
            />
        </div>
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct AddTaskProps {
    pub on_add: Callback<String>,
}

#[function_component(AddTask)]
fn add_task(props: &AddTaskProps) -> Html {
    let description = use_state(|| "".to_string());
    let on_add = props.on_add.clone();

    let onsubmit = {
        let description = description.clone();
        let on_add = on_add.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let desc = (*description).clone();
            if !desc.is_empty() {
                on_add.emit(desc.clone());
                description.set("".to_string());
            }
        })
    };

    let oninput = {
        let description = description.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            description.set(input.value());
        })
    };

    html! {
        <form {onsubmit}>
            <input
                type="text"
                value={(*description).clone()}
                oninput={oninput}
                placeholder="Створити нове завдання"
            />
            <button type="submit">{ "Додати" }</button>
        </form>
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct TaskListProps {
    pub tasks: Vec<Task>,
    pub on_toggle: Callback<u32>,
    pub on_edit: Callback<(u32, String)>,
    pub on_delete: Callback<u32>,
}

#[function_component(TaskList)]
fn task_list(props: &TaskListProps) -> Html {
    let tasks = props.tasks.clone();

    html! {
        <ul>
            { for tasks.iter().map(|task| {
                html! {
                    <TaskItem
                        key={task.id}
                        task={task.clone()}
                        on_toggle={props.on_toggle.clone()}
                        on_edit={props.on_edit.clone()}
                        on_delete={props.on_delete.clone()}
                    />
                }
            }) }
        </ul>
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct TaskItemProps {
    pub task: Task,
    pub on_toggle: Callback<u32>,
    pub on_edit: Callback<(u32, String)>,
    pub on_delete: Callback<u32>,
}

#[function_component(TaskItem)]
fn task_item(props: &TaskItemProps) -> Html {
    let TaskItemProps { task, on_toggle, on_edit, on_delete } = props.clone();

    let is_editing = use_state(|| false);
    let new_description = use_state(|| task.description.clone());

    let start_editing = {
        let is_editing = is_editing.clone();
        let new_description = new_description.clone();
        let task_description = task.description.clone();
        Callback::from(move |_| {
            is_editing.set(true);
            new_description.set(task_description.clone());
        })
    };

    let save_edit = {
        let is_editing = is_editing.clone();
        let new_description = new_description.clone();
        let on_edit = on_edit.clone();
        let task_id = task.id;
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if !(*new_description).is_empty() {
                on_edit.emit((task_id, (*new_description).clone()));
                is_editing.set(false);
            }
        })
    };

    let handle_input = {
        let new_description = new_description.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_description.set(input.value());
        })
    };

    let cancel_editing = {
        let is_editing = is_editing.clone();
        Callback::from(move |_| {
            is_editing.set(false);
        })
    };

    html! {
        <li class={if task.completed { "completed" } else { "" }} key={task.id}>
            { if *is_editing {
                html! {
                    <form onsubmit={save_edit.clone()}>
                        <input
                            type="text"
                            value={(*new_description).clone()}
                            oninput={handle_input}
                            placeholder="Редагувати"
                        />
                        <div class="actions">
                            <button type="submit">{ "Зберегти" }</button>
                            <button type="button" onclick={cancel_editing}>{ "Відмінити" }</button>
                        </div>
                    </form>
                }
            } else {
                html! {
                    <>
                        <label>
                            <input
                                type="checkbox"
                                checked={task.completed}
                                onchange={on_toggle.reform(move |_| task.id)}
                            />
                            { &task.description }
                        </label>
                        <div class="actions">
                            <button onclick={start_editing}>{ "Редагувати" }</button>
                            <button onclick={on_delete.reform(move |_| task.id)}>{ "Видалити" }</button>
                        </div>
                    </>
                }
            }}
        </li>
    }
}
