class Good { operator fun inc(): Good = this }
class Holder { operator fun get(index: Int): Good = Good(); operator fun set(index: Int, value: Good) {} }
fun use(value: Holder): Good = ++value[0]
