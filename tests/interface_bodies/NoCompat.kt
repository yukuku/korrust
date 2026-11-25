package nocompat
interface Api { fun <T> echo(value: T): T = value }
