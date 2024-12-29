use std::io;

fn main() {
    println!("Введіть 'exit' щоб вийти.");
    println!("Введіть 'pn' для використання польської нотації (наприклад, pn + 3 5).");

    let mut current_result: f64 = 0.0;

    loop {
        println!("\nРезультат: {}", current_result);
        println!("Введіть операцію та цифру (наприклад, + 5):");

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") {
            println!("Вихід з калькулятору... На все добре!");
            break;
        }

        if input.starts_with("pn ") {
            let expression = &input[3..];
            match evaluate_polish_notation(expression) {
                Ok(result) => {
                    current_result = result;
                    println!("Результат: {}", current_result);
                }
                Err(e) => println!("Помилка: {}", e),
            }
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.len() != 2 {
            println!("Неправильні вхідні дані. Будь-ласка, введіть операцію та цифру (наприклад, + 5).");
            continue;
        }

        let operation = parts[0];
        let number: f64 = match parts[1].parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Неправильна цифра: {}. Спробуйте ще раз.", parts[1]);
                continue;
            }
        };

        match operation {
            "+" => current_result += number,
            "-" => current_result -= number,
            "*" => current_result *= number,
            "/" => {
                if number == 0.0 {
                    println!("Ділення на нуль не дозволено. Спробуйте ще раз.");
                    continue;
                }
                current_result /= number;
            }
            _ => {
                println!("Невідома операція: {}. Використовуйте +, -, *, або /.", operation);
                continue;
            }
        }

        println!("Result: {}", current_result);
    }
}

fn evaluate_polish_notation(expression: &str) -> Result<f64, String> {
    let mut stack: Vec<f64> = Vec::new();

    for token in expression.split_whitespace().rev() {
        if let Ok(num) = token.parse::<f64>() {
            stack.push(num);
        } else {
            if stack.len() < 2 {
                return Err("Недостатньо операндів для обчислення".to_string());
            }

            let a = stack.pop().unwrap();
            let b = stack.pop().unwrap();

            let result = match token {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" => {
                    if b == 0.0 {
                        return Err("Ділення на нуль".to_string());
                    }
                    a / b
                }
                _ => return Err(format!("Невідомий оператор: {}", token)),
            };

            stack.push(result);
        }
    }

    if stack.len() != 1 {
        return Err("Неправильний вираз".to_string());
    }

    Ok(stack.pop().unwrap())
}
