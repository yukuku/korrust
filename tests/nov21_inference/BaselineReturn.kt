package baseline
fun early(flag: Boolean): Array<String> {
    if (flag) return emptyArray()
    return arrayOf("value")
}
