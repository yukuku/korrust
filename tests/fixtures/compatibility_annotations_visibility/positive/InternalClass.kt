package compatibility.visibility

class InternalClass {
    internal fun internalValue(value: Int): Int = value + 19

    fun publicValue(value: Int): Int = internalValue(value)
}
