package friends.library
class PublicValue(val value: Int)
fun shadow(): Int {
    val Plain = PublicValue(12)
    return Plain.value
}
