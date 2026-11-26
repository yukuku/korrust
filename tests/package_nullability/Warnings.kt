package defaults.consumer
fun warningNullable(): String? = warning.Library.echo(null)
fun warningRequired(): String = warning.Library.echo(null)
fun warningInferred(): Int { var value = warning.Library.echo("value"); value = null; return if (value == null) 1 else 2 }
fun warningGlobal(): String? = globalwarning.Library.echo(null)
