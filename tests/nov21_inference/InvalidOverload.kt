package invalid
import inference.dependency.selected
fun bad(value: String?): Int {
    if (selected(value, 0)) return value.length
    return 0
}
