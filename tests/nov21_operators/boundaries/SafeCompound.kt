class Holder(var item: Int)
fun use(value: Holder?) { value?.item += 1 }
