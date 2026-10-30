package objectcases

sealed interface Command

data object Start : Command

data object Stop : Command {
    val message: String = "stop"
    override fun toString(): String = message
}

object Container {
    data object Nested : Command
}

fun describe(command: Command): String = when (command) {
    Start -> "start"
    Stop -> "stop"
    Container.Nested -> "nested"
}

object Counter {
    var count: Int = 0
}

fun effect(): Int {
    Counter.count = Counter.count + 1
    return Counter.count
}

data object Tracked {
    val first: Int = effect()
    init { Counter.count = Counter.count + 10 }
    val second: Int = effect()
    fun value(): Int = first + second
    fun equals(value: Int): Boolean = value == 7
}

data object Empty
object Ordinary

data object SerializableEntry : java.io.Serializable
