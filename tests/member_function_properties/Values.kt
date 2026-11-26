package memberfunctions

interface Request {
    fun execute(): Int
}

class Captured(initial: Int) {
    private var action: (Int) -> Int = { it + initial }
    fun replace(delta: Int) { action = { it * delta } }
    fun request(key: Int): Request = object : Request {
        override fun execute(): Int = action(key)
    }
}

class Target {
    var trace: Int = 0
    val action: (Int) -> Int
        get() {
            trace = trace * 10 + 2
            return { value -> trace = trace * 10 + 4; value + 10 }
        }
    fun action(value: String): Int = value.length + 100
    fun own(): Int = action(5)
}

fun receiver(target: Target): Target { target.trace = target.trace * 10 + 1; return target }
fun argument(target: Target): Int { target.trace = target.trace * 10 + 3; return 5 }
fun order(target: Target): Int = receiver(target).action(argument(target))
fun nullable(target: Target?): Int? = target?.action(7)
fun Target.action(value: Int): Int = value + 200

class Both {
    val action: (Int) -> Int = { it + 30 }
    fun action(value: Int): Int = value + 40
    fun own(): Int = action(2)
}

interface Dispatch {
    val action: (Int) -> Int
}

class Multiplied(factor: Int) : Dispatch {
    override val action: (Int) -> Int = { it * factor }
}

fun dispatch(value: Dispatch): Int = value.action(3)
class Generic<T>(val action: (T) -> T)
fun generic(value: Generic<String>): String = value.action("value")
class ExtensionHolder(val action: String.(Int) -> Int)
fun extension(value: ExtensionHolder): Int = value.action("abc", 2)
