package fieldconsumer
import fieldannotations.Fields

fun text(): String { Fields.text = "changed"; return Fields.text }
fun instance(value: Fields): String { value.item = value.item + "+"; return value.item }
fun nonnull(): String = Fields.nonnull
fun nullable(): String? = Fields.nullable
