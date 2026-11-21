package discrepancy
object Trace { var text: String = "" }
class Value { operator fun inc(): Value { Trace.text = Trace.text + "+"; return Value() } }
class Holder {
    var saved: Value = Value()
    var item: Value
        get() { Trace.text = Trace.text + "g"; return saved }
        set(value) { Trace.text = Trace.text + "s"; saved = value }
}
fun main() { val holder = Holder(); val result = ++holder.item; println(Trace.text) }
