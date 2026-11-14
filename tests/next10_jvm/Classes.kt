package nextjvm.api

class Secret private constructor(val value: Int) {
    fun again(value: Int): Secret = Secret(value)
    fun factory(): (Int) -> Secret = { Secret(it) }
    class Nested {
        fun create(value: Int): Secret = Secret(value)
    }
    companion object {
        fun create(value: Int): Secret = Secret(value)
    }
}

class ModuleOnly internal constructor(val value: Int)
fun moduleValue(): Int = ModuleOnly(9).value

class ObjectSecret private constructor() {
    object Factory {
        fun create(): ObjectSecret = ObjectSecret()
    }
}
