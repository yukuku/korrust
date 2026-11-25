interface Base
fun bad(): Base = object : Base() {}
