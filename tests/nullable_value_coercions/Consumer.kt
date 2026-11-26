package nullablevalues.consumer
import nullablevalues.library.Mode
import nullablevalues.library.LongValue
import nullablevalues.library.Text
import nullablevalues.library.NullableText
import nullablevalues.library.NullableNumber
import nullablevalues.library.Holder
import nullablevalues.library.Values
import nullablevalues.library.mode
import nullablevalues.library.longValue
import nullablevalues.library.text
import nullablevalues.library.nullableText
import nullablevalues.library.nullText
import nullablevalues.library.nullNumber
import nullablevalues.library.defaultMode
import nullablevalues.library.optionalMode
import nullablevalues.library.defaultNullText
import nullablevalues.library.optionalNullText
import nullablevalues.library.defaultNullNumber
import nullablevalues.library.optionalNullNumber
import nullablevalues.library.both
import nullablevalues.library.mixed
import nullablevalues.library.genericDefault
import nullablevalues.library.identity
import nullablevalues.library.inspectMode
import nullablevalues.library.inspectLong
import nullablevalues.library.inspectText
import nullablevalues.library.inspectNullText
import nullablevalues.library.inspectNullNumber
fun direct(): Int = inspectMode(mode())
fun local(): Int { val value: Mode? = mode(); return inspectMode(value) }
fun property(): Int = inspectMode(Values.mode)
fun constructed(): Int = inspectMode(Mode(23))
fun generic(): Int = inspectMode(identity(mode()))
fun container(): Int = inspectMode(Holder<Mode?>(mode()).value)
fun defaultValue(): Int = inspectMode(defaultMode())
fun optionalDefault(): Int = inspectMode(optionalMode())
fun explicitNull(): Int = inspectMode(optionalMode(null))
fun wide(): Long = inspectLong(longValue())
fun reference(): String = inspectText(text())
fun referenceNullSentinel(): String = inspectText(nullableText(false))
fun referencePresent(): String = inspectText(nullableText(true))
fun nullCarrier(): String = inspectNullText(nullText())
fun nullCarrierProperty(): String = inspectNullText(Values.nullText)
fun nullCarrierDefault(): String = inspectNullText(defaultNullText())
fun nullCarrierOptionalDefault(): String = inspectNullText(optionalNullText())
fun nullCarrierExplicitNull(): String = inspectNullText(optionalNullText(null))
fun nullCarrierGeneric(): String = inspectNullText(identity(nullText()))
fun nullCarrierContainer(): String = inspectNullText(Holder<NullableText?>(nullText()).value)
fun numberCarrier(): String = inspectNullNumber(nullNumber())
fun numberCarrierProperty(): String = inspectNullNumber(Values.nullNumber)
fun numberCarrierDefault(): String = inspectNullNumber(defaultNullNumber())
fun numberCarrierOptionalDefault(): String = inspectNullNumber(optionalNullNumber())
fun numberCarrierExplicitNull(): String = inspectNullNumber(optionalNullNumber(null))
fun numberCarrierGeneric(): String = inspectNullNumber(identity(nullNumber()))
fun numberCarrierContainer(): String = inspectNullNumber(Holder<NullableNumber?>(nullNumber()).value)
fun constructorTextDefault(): String = inspectNullText(nullablevalues.library.TextHolder().value)
fun constructorNumberDefault(): String = inspectNullNumber(nullablevalues.library.NumberHolder().value)
fun mixedDefault(): String = inspectNullText(mixed(nullText()))
fun genericSentinelDefault(): String = inspectNullText(genericDefault<NullableText>())
fun genericSentinelProvided(): String = inspectNullText(genericDefault<NullableText>(nullText()))
fun constructorProvidedSentinel(): String = inspectNullText(nullablevalues.library.MixedHolder(nullText()).value)
fun providedSentinelDefault(): String = inspectNullText(both(nullText()))
