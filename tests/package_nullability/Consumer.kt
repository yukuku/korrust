package defaults.consumer
import nonnull.kotlinNullable
fun nonNull(): String = nonnull.Library.echo("ok")
fun nullableValue(): String? = nullable.Library.echo(null)
fun explicitNullable(): String? = nonnull.Library.maybe(null)
fun explicitRequired(): String = nullable.Library.required("required")
fun platformResult(): String? = parameters.Library.missing("input")
fun values(): Int = nonnull.Library.words()[0].length + nonnull.Library.array()[0].length
fun fields(): Int = nonnull.Library().field.length + (nonnull.Library().maybeField?.length ?: 2)
fun nullableField(): String? = nullable.Library().field
fun nullableElement(): String? = deepnullable.Library.words()!![0]
fun nullableArrayElement(): String? = deepnullable.Library.array()!![0]
fun qualifierAlias(): String = nickname.Library.read()
fun kotlinMetadataWins(): String? = kotlinNullable(null)
