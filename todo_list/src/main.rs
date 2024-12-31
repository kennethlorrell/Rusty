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
            let mut new_tasks = tasks.to_vec();
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
            let mut new_tasks = tasks.to_vec();
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

    html! {
        <div class="todo-app">
            <h1>{ "To-Do List" }</h1>
            <AddTask on_add={add_task} />
            <TaskList tasks={(*tasks).clone()} on_toggle={toggle_task} on_delete={delete_task} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
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
            let desc = description.to_string();
            if !desc.is_empty() {
                on_add.emit(desc.clone());
                description.set("".to_string());
            }
        })
    };

    html! {
        <form {onsubmit}>
            <input
                type="text"
                value={(*description).clone()}
                oninput={Callback::from(move |e: InputEvent| {
                    description.set(e.target_unchecked_into::<web_sys::HtmlInputElement>().value());
                })}
            />
            <button type="submit">{ "Додати" }</button>
        </form>
    }
}

#[derive(Properties, PartialEq)]
pub struct TaskListProps {
    pub tasks: Vec<Task>,
    pub on_toggle: Callback<u32>,
    pub on_delete: Callback<u32>,
}

#[function_component(TaskList)]
fn task_list(props: &TaskListProps) -> Html {
    let tasks = props.tasks.clone();

    html! {
        <ul>
            { for tasks.iter().map(|task| {
                let task_id = task.id;
                let task_class = if task.completed { "completed" } else { "" };

                html! {
                    <li class={task_class} key={task.id}>
                        <label>
                            <input
                                type="checkbox"
                                checked={task.completed}
                                onchange={props.on_toggle.reform(move |_| task_id)}
                            />
                            { &task.description }
                        </label>
                        <button onclick={props.on_delete.reform(move |_| task_id)}>{ "Видалити" }</button>
                    </li>
                }
            }) }
        </ul>
    }
}
