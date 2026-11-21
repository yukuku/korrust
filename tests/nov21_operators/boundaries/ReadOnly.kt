class Good { operator fun inc(): Good = this }
class Holder(val item: Good)
fun use(value: Holder) { value.item++ }
