class MutableReceiver : Receiver() {
    override fun accept(values: MutableList<String>?): Int = values?.size ?: -1
}
fun readOnly(): Receiver = object : Receiver() {
    override fun accept(values: List<String?>): Int = values.size
}
