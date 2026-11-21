package invalid
fun isNullOrEmpty(value: String?): Boolean = value == null
fun bad(value: String?): Int {
    if (isNullOrEmpty(value)) return -1
    return value.length
}
