open class Base { fun value(): Int = 1 }
fun bad(): Base = object : Base() { override fun value(): Int = 2 }
