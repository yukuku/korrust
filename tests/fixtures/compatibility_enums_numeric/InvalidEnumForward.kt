package compatibility.invalid
enum class Forward(val number: Int) { A(B.number), B(2) }
