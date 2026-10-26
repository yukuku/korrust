package compatibility.diagnostics

enum class WithBody {
    ITEM {
        override fun value(): Int = 3
    };
    open fun value(): Int = 1
}
