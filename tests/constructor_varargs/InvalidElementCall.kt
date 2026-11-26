package constructorvarargs
class Strings(vararg val values: String)
fun invalid(): Strings = Strings("a", 1)
