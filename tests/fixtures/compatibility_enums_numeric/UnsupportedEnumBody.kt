package compatibility.unsupported
enum class EntryBody {
    A { override fun answer(): Int = 1 };
    abstract fun answer(): Int
}
