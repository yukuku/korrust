class Good { operator fun inc(): Good = this }
class Holder(var item: Good)
fun use(value: Holder?) { value?.item++ }
