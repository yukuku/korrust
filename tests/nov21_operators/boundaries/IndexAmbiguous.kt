class Both { operator fun plusAssign(value: Int) {}
operator fun plus(value: Int): Both = this }
class Holder { operator fun get(index: Int): Both = Both(); operator fun set(index: Int, value: Both) {} }
fun use(value: Holder) { value[0] += 1 }
