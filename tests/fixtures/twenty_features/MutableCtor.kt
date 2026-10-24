package twenty
class MutableCtor(var count: Int, var text: String?, private var hidden: Long) {
    fun update(value: Int): Int { count = value; return count }
    fun updateHidden(value: Long): Long { hidden = value; return hidden }
}
