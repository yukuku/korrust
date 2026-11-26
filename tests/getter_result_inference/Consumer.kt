import java.util.ArrayDeque
class TaskQueue(val counter: Counter) {
    private val tasks = ArrayDeque<String>()
    val size get() = tasks.size
    val next get() = counter.next()
    fun add(s: String) { tasks.addLast(s) }
}
val top get() = 42
