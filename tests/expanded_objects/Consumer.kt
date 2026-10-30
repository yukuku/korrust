package objectconsumer
import objectcases.*
fun consume(): String {
    check(Empty.toString() == "Empty")
    check(Start != Stop)
    check(Start.hashCode() == "objectcases.Start".hashCode())
    fun closed(value: Command): String = when(value) {
        Start -> "start"
        Stop -> "stop"
        Container.Nested -> "nested"
    }
    check(closed(Start) == "start")
    check(closed(Stop) == "stop")
    check(closed(Container.Nested) == "nested")
    return "OK"
}
