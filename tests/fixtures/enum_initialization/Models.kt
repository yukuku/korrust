package enums.initialization

object Events {
    var text: String = ""
    fun mark(name: String, value: Int): Int { text += name; return value }
    fun fail(name: String): Int { text += name; throw IllegalArgumentException(name) }
}

enum class BadDefault(val value: Int = Events.fail("default")) {
    A { init { Events.mark("unreachable-body", 0) } },
    B(Events.mark("unreachable-entry", 2))
}

enum class BadBody(val value: Int) {
    A(Events.mark("argument", 1)) { init { Events.fail("body") } },
    B(Events.mark("unreachable-second", 2))
}

interface NullableAction { fun apply(value: String?): String? }
enum class Action : NullableAction {
    A {
        override fun apply(value: String?): String? {
            if (value == null) return null
            if (value == "fail") throw IllegalStateException(value)
            return value + "!"
        }
    };
    abstract override fun apply(value: String?): String?
}
