package compatibility.invalid
enum class CompanionForward(val number: Int) {
    A(next());
    companion object { fun next(): Int = 1 }
}
