package fixtures
class Slot<T> {
    var value: T? = null
}
inline fun <reified T> slot(): Slot<T> = Slot()
