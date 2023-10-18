#[wasm_bindgen]
pub fn derivative(equation: &str) -> String {
    equation.replace("()", "")
}
