package constructorvarargs
class InvalidElementWrite(vararg val values: String) {
    fun write() { values[0] = "changed" }
}
