import scoped.runValue
fun answer(value: String): Int {
    value.runValue { text ->
        text.length
        return text.length
    }
}
