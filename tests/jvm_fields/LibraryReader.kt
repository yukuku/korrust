package fieldreader
import fieldapi.Capsule
fun update(value: Capsule): String {
    value.count = value.count + 1
    value.label = "korrust"
    return value.name + value.count + value.label + value.ordinary
}
