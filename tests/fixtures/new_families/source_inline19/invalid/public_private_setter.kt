package settervisibility
var state: Int = 0
    private set
inline fun write(value: Int) { state = value }
