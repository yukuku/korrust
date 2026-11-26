import nestedapi.Outer
import nestedapi.Provider
fun nullable(missing: Boolean): Outer.Leaf? = Provider.nullable(missing)
fun nonNull(): Outer.Leaf = Provider.nonNull()
fun nested(missing: Boolean): Outer.Middle.Leaf? = Provider.nested(missing)
fun acceptNull(): Int = Provider.accept(null)
fun accept(value: Outer.Leaf?): Int = Provider.accept(value)
fun value(missing: Boolean): Int = Provider.nullable(missing)?.value ?: -1
fun field(value: Provider): Outer.Leaf? = value.field
fun write(value: Provider, item: Outer.Leaf?) { value.field = item }
fun staticField(): Outer.Middle.Leaf? = Provider.staticField
fun writeStatic(value: Outer.Middle.Leaf?) { Provider.staticField = value }
