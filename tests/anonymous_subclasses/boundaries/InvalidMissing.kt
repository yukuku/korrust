abstract class Base { abstract fun value(): Int }
fun bad(): Base = object : Base() {}
