package compatibility.enumsnumeric

object EnumTrace {
    var text: String = ""
    fun note(value: Int): Int {
        text += value.toString()
        return value
    }
}

enum class Signal(val code: Int, private val label: String) {
    RED(EnumTrace.note(1), "red"),
    GREEN(EnumTrace.note(2), "green");

    val doubled: Int = code * 2
    init { EnumTrace.note(code + 4) }
    fun caption(): String = label + ":" + code

    companion object {
        val initialized: Int = EnumTrace.note(9)
        fun fromCode(value: Int): Signal = if (value == 1) RED else GREEN
    }
}

enum class Empty {
    ;
    fun count(): Int = 0
}
