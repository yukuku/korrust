open class Base { open fun n(): Int = 1 }
fun bad(): Base = object : Base() { override fun n(): Int = 2 }
