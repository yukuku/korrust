import delegated.*
fun scalarConsumer(value: PlainIntForward): Int = value.value
fun stringConsumer(value: RefForward): String = value.value
fun genericConsumer(value: Forward<String>): String = value.echo(value.value)
fun inheritedConsumer(value: Inherited<String>): String = value.echo("O")
fun counterConsumer(value: Saved): Int {
    value.count = 13
    return value.read() + value.count
}
fun platformMetadataConsumer(value: JavaForward): String? = value.echo(null)
fun floatConsumer(value: FloatForward): Float = value.value
fun doubleConsumer(value: DoubleForward): Double = value.value
fun byteConsumer(value: ByteForward): Byte = value.value
