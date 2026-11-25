open class Base
fun bad(): Base = object : Base() { val extra: Int = 2 }
