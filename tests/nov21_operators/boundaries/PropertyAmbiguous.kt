class Both { operator fun plusAssign(value: Int) {}
operator fun plus(value: Int): Both = this }
class Holder(var item: Both)
fun use(value: Holder) { value.item += 1 }
