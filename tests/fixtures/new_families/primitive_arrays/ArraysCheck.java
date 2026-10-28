package newfamilies.arrays;
import java.util.Arrays;
import kotlin.jvm.functions.Function1;
public final class ArraysCheck {
static int checks; static int calls; static String trace;
static void require(boolean value) { checks++; if (!value) throw new AssertionError("check=" + checks); }
public static void main(String[] args) {
long[] zerosLong = ArraysKt.defaultsLong(3);
require(zerosLong.length == 3 && zerosLong[0] == 0L && zerosLong[2] == 0L);
calls = 0; trace = "";
Function1<Integer, Long> initLong = index -> { calls++; trace += index; return (long) index + 1L; };
long[] valuesLong = ArraysKt.initLong(3, initLong);
require(calls == 3 && trace.equals("012"));
require(ArraysKt.keepLong(valuesLong) == valuesLong);
require(ArraysKt.nullableLong(valuesLong) == valuesLong);
require(ArraysKt.nullableLong(null).length == 0);
long previousLong = valuesLong[1];
require(ArraysKt.putLong(valuesLong, 1, 77L) == previousLong);
require(ArraysKt.getLong(valuesLong, 1) == 77L);
long[] packedLong = ArraysKt.packLong(1L, 2L);
require(Arrays.equals(packedLong, new long[] {1L, 2L}));
require(Arrays.equals(ArraysKt.builtinLong(), new long[] {1L, 2L, 3L}));
long[] spreadLong = ArraysKt.spreadLong(packedLong);
require(Arrays.equals(spreadLong, new long[] {1L, 1L, 2L, 3L}));
require(spreadLong != packedLong);
require(Arrays.equals(ArraysKt.copyLong(packedLong, 3), new long[] {1L, 2L, 0L}));
calls = 0;
require(ArraysKt.initLong(0, initLong).length == 0 && calls == 0);
try { ArraysKt.initLong(-1, initLong); throw new AssertionError(); } catch (NegativeArraySizeException expected) { require(calls == 0); }
try { ArraysKt.getLong(valuesLong, -1); throw new AssertionError(); } catch (ArrayIndexOutOfBoundsException expected) { require(true); }
try { ArraysKt.putLong(valuesLong, valuesLong.length, 1L); throw new AssertionError(); } catch (ArrayIndexOutOfBoundsException expected) { require(true); }
try { ArraysKt.keepLong(null); throw new AssertionError(); } catch (NullPointerException expected) { require(true); }
ArrayEffects.reset(false);
ArraysKt.orderedLong();
require(ArrayEffects.trace.equals("AIV") && ArrayEffects.longValues[0] == 77L);
ArrayEffects.reset(true);
try { ArraysKt.orderedLong(); throw new AssertionError(); } catch (NullPointerException expected) { require(ArrayEffects.trace.equals("AIV")); }
boolean[] zerosBoolean = ArraysKt.defaultsBoolean(3);
require(zerosBoolean.length == 3 && zerosBoolean[0] == false && zerosBoolean[2] == false);
calls = 0; trace = "";
Function1<Integer, Boolean> initBoolean = index -> { calls++; trace += index; return index % 2 == 0; };
boolean[] valuesBoolean = ArraysKt.initBoolean(3, initBoolean);
require(calls == 3 && trace.equals("012"));
require(ArraysKt.keepBoolean(valuesBoolean) == valuesBoolean);
require(ArraysKt.nullableBoolean(valuesBoolean) == valuesBoolean);
require(ArraysKt.nullableBoolean(null).length == 0);
boolean previousBoolean = valuesBoolean[1];
require(ArraysKt.putBoolean(valuesBoolean, 1, true) == previousBoolean);
require(ArraysKt.getBoolean(valuesBoolean, 1) == true);
boolean[] packedBoolean = ArraysKt.packBoolean(true, false);
require(Arrays.equals(packedBoolean, new boolean[] {true, false}));
require(Arrays.equals(ArraysKt.builtinBoolean(), new boolean[] {true, false, true}));
boolean[] spreadBoolean = ArraysKt.spreadBoolean(packedBoolean);
require(Arrays.equals(spreadBoolean, new boolean[] {true, true, false, true}));
require(spreadBoolean != packedBoolean);
require(Arrays.equals(ArraysKt.copyBoolean(packedBoolean, 3), new boolean[] {true, false, false}));
calls = 0;
require(ArraysKt.initBoolean(0, initBoolean).length == 0 && calls == 0);
try { ArraysKt.initBoolean(-1, initBoolean); throw new AssertionError(); } catch (NegativeArraySizeException expected) { require(calls == 0); }
try { ArraysKt.getBoolean(valuesBoolean, -1); throw new AssertionError(); } catch (ArrayIndexOutOfBoundsException expected) { require(true); }
try { ArraysKt.putBoolean(valuesBoolean, valuesBoolean.length, true); throw new AssertionError(); } catch (ArrayIndexOutOfBoundsException expected) { require(true); }
try { ArraysKt.keepBoolean(null); throw new AssertionError(); } catch (NullPointerException expected) { require(true); }
ArrayEffects.reset(false);
ArraysKt.orderedBoolean();
require(ArrayEffects.trace.equals("AIV") && ArrayEffects.booleanValues[0] == true);
ArrayEffects.reset(true);
try { ArraysKt.orderedBoolean(); throw new AssertionError(); } catch (NullPointerException expected) { require(ArrayEffects.trace.equals("AIV")); }
float[] zerosFloat = ArraysKt.defaultsFloat(3);
require(zerosFloat.length == 3 && zerosFloat[0] == 0f && zerosFloat[2] == 0f);
calls = 0; trace = "";
Function1<Integer, Float> initFloat = index -> { calls++; trace += index; return index + 1.25f; };
float[] valuesFloat = ArraysKt.initFloat(3, initFloat);
require(calls == 3 && trace.equals("012"));
require(ArraysKt.keepFloat(valuesFloat) == valuesFloat);
require(ArraysKt.nullableFloat(valuesFloat) == valuesFloat);
require(ArraysKt.nullableFloat(null).length == 0);
float previousFloat = valuesFloat[1];
require(ArraysKt.putFloat(valuesFloat, 1, 77.25f) == previousFloat);
require(ArraysKt.getFloat(valuesFloat, 1) == 77.25f);
float[] packedFloat = ArraysKt.packFloat(1.25f, 2.5f);
require(Arrays.equals(packedFloat, new float[] {1.25f, 2.5f}));
require(Arrays.equals(ArraysKt.builtinFloat(), new float[] {1.25f, 2.5f, 3.75f}));
float[] spreadFloat = ArraysKt.spreadFloat(packedFloat);
require(Arrays.equals(spreadFloat, new float[] {1.25f, 1.25f, 2.5f, 3.75f}));
require(spreadFloat != packedFloat);
require(Arrays.equals(ArraysKt.copyFloat(packedFloat, 3), new float[] {1.25f, 2.5f, 0f}));
calls = 0;
require(ArraysKt.initFloat(0, initFloat).length == 0 && calls == 0);
try { ArraysKt.initFloat(-1, initFloat); throw new AssertionError(); } catch (NegativeArraySizeException expected) { require(calls == 0); }
try { ArraysKt.getFloat(valuesFloat, -1); throw new AssertionError(); } catch (ArrayIndexOutOfBoundsException expected) { require(true); }
try { ArraysKt.putFloat(valuesFloat, valuesFloat.length, 1.25f); throw new AssertionError(); } catch (ArrayIndexOutOfBoundsException expected) { require(true); }
try { ArraysKt.keepFloat(null); throw new AssertionError(); } catch (NullPointerException expected) { require(true); }
ArrayEffects.reset(false);
ArraysKt.orderedFloat();
require(ArrayEffects.trace.equals("AIV") && ArrayEffects.floatValues[0] == 77.25f);
ArrayEffects.reset(true);
try { ArraysKt.orderedFloat(); throw new AssertionError(); } catch (NullPointerException expected) { require(ArrayEffects.trace.equals("AIV")); }
double[] zerosDouble = ArraysKt.defaultsDouble(3);
require(zerosDouble.length == 3 && zerosDouble[0] == 0d && zerosDouble[2] == 0d);
calls = 0; trace = "";
Function1<Integer, Double> initDouble = index -> { calls++; trace += index; return index + 1.25; };
double[] valuesDouble = ArraysKt.initDouble(3, initDouble);
require(calls == 3 && trace.equals("012"));
require(ArraysKt.keepDouble(valuesDouble) == valuesDouble);
require(ArraysKt.nullableDouble(valuesDouble) == valuesDouble);
require(ArraysKt.nullableDouble(null).length == 0);
double previousDouble = valuesDouble[1];
require(ArraysKt.putDouble(valuesDouble, 1, 77.5) == previousDouble);
require(ArraysKt.getDouble(valuesDouble, 1) == 77.5);
double[] packedDouble = ArraysKt.packDouble(1.25, 2.5);
require(Arrays.equals(packedDouble, new double[] {1.25, 2.5}));
require(Arrays.equals(ArraysKt.builtinDouble(), new double[] {1.25, 2.5, 3.75}));
double[] spreadDouble = ArraysKt.spreadDouble(packedDouble);
require(Arrays.equals(spreadDouble, new double[] {1.25, 1.25, 2.5, 3.75}));
require(spreadDouble != packedDouble);
require(Arrays.equals(ArraysKt.copyDouble(packedDouble, 3), new double[] {1.25, 2.5, 0d}));
calls = 0;
require(ArraysKt.initDouble(0, initDouble).length == 0 && calls == 0);
try { ArraysKt.initDouble(-1, initDouble); throw new AssertionError(); } catch (NegativeArraySizeException expected) { require(calls == 0); }
try { ArraysKt.getDouble(valuesDouble, -1); throw new AssertionError(); } catch (ArrayIndexOutOfBoundsException expected) { require(true); }
try { ArraysKt.putDouble(valuesDouble, valuesDouble.length, 1.25); throw new AssertionError(); } catch (ArrayIndexOutOfBoundsException expected) { require(true); }
try { ArraysKt.keepDouble(null); throw new AssertionError(); } catch (NullPointerException expected) { require(true); }
ArrayEffects.reset(false);
ArraysKt.orderedDouble();
require(ArrayEffects.trace.equals("AIV") && ArrayEffects.doubleValues[0] == 77.5);
ArrayEffects.reset(true);
try { ArraysKt.orderedDouble(); throw new AssertionError(); } catch (NullPointerException expected) { require(ArrayEffects.trace.equals("AIV")); }
require(ArraysKt.sumLong(new long[] {Long.MAX_VALUE, 1L}) == Long.MIN_VALUE);
require(ArraysKt.countBoolean(new boolean[] {false, true, true}) == 2);
require(ArraysKt.sumFloat(new float[] {1.25f, 2.5f}) == 3.75f);
require(ArraysKt.sumDouble(new double[] {1.25, 2.5}) == 3.75);
float[] floatBits = new float[] {1f}; ArraysKt.putFloat(floatBits, 0, -0.0f); require(Float.floatToRawIntBits(floatBits[0]) == 0x80000000);
double[] doubleBits = new double[] {1d}; ArraysKt.putDouble(doubleBits, 0, -0.0); require(Double.doubleToRawLongBits(doubleBits[0]) == 0x8000000000000000L);
ArraysKt.putFloat(floatBits, 0, Float.NaN); require(Float.isNaN(ArraysKt.getFloat(floatBits, 0)));
ArraysKt.putDouble(doubleBits, 0, Double.NaN); require(Double.isNaN(ArraysKt.getDouble(doubleBits, 0)));
IllegalStateException failureLong = new IllegalStateException("identityLong"); calls = 0;
try { ArraysKt.initLong(3, index -> { calls++; if (index == 1) throw failureLong; return 0L; }); throw new AssertionError(); } catch (IllegalStateException actual) { require(actual == failureLong && calls == 2); }
calls = 0; try { ArraysKt.initLong(2, index -> { calls++; return null; }); throw new AssertionError(); } catch (NullPointerException actual) { require(calls == 1); }
IllegalStateException failureBoolean = new IllegalStateException("identityBoolean"); calls = 0;
try { ArraysKt.initBoolean(3, index -> { calls++; if (index == 1) throw failureBoolean; return false; }); throw new AssertionError(); } catch (IllegalStateException actual) { require(actual == failureBoolean && calls == 2); }
calls = 0; try { ArraysKt.initBoolean(2, index -> { calls++; return null; }); throw new AssertionError(); } catch (NullPointerException actual) { require(calls == 1); }
IllegalStateException failureFloat = new IllegalStateException("identityFloat"); calls = 0;
try { ArraysKt.initFloat(3, index -> { calls++; if (index == 1) throw failureFloat; return 0f; }); throw new AssertionError(); } catch (IllegalStateException actual) { require(actual == failureFloat && calls == 2); }
calls = 0; try { ArraysKt.initFloat(2, index -> { calls++; return null; }); throw new AssertionError(); } catch (NullPointerException actual) { require(calls == 1); }
IllegalStateException failureDouble = new IllegalStateException("identityDouble"); calls = 0;
try { ArraysKt.initDouble(3, index -> { calls++; if (index == 1) throw failureDouble; return 0d; }); throw new AssertionError(); } catch (IllegalStateException actual) { require(actual == failureDouble && calls == 2); }
calls = 0; try { ArraysKt.initDouble(2, index -> { calls++; return null; }); throw new AssertionError(); } catch (NullPointerException actual) { require(calls == 1); }
require(Arrays.equals(AliasedFactoryKt.renamedFactory(), new double[] {1.25, 2.5}));
require(newfamilies.arrays.shadow.ShadowedKt.customFactory()[0] == 99.0);
require(Arrays.equals(newfamilies.arrays.shadow.ShadowedKt.customConstructor(), new long[] {99L}));
require(ArraysKt.floatTemplate(-0.0f).equals("[-0.0]"));
require(ArraysKt.floatTemplate(Float.NaN).equals("[NaN]"));
require(ArraysKt.doubleTemplate(Double.POSITIVE_INFINITY).equals("[Infinity]"));
require(ArraysKt.doubleTemplate(null).equals("[null]"));
require(ArraysKt.byteTemplate(Byte.MIN_VALUE).equals("[-128]"));
ArrayHolder holder = new ArrayHolder(valuesLong, valuesBoolean, valuesFloat, valuesDouble);
require(holder.getLongs() == valuesLong && holder.getBooleans() == valuesBoolean);
require(holder.getFloats() == valuesFloat && holder.getDoubles() == valuesDouble);
long[] singleLong = ArraysKt.spreadOnlyLong(packedLong);
require(singleLong != packedLong && Arrays.equals(singleLong, packedLong));
boolean[] singleBoolean = ArraysKt.spreadOnlyBoolean(packedBoolean);
require(singleBoolean != packedBoolean && Arrays.equals(singleBoolean, packedBoolean));
float[] singleFloat = ArraysKt.spreadOnlyFloat(packedFloat);
require(singleFloat != packedFloat && Arrays.equals(singleFloat, packedFloat));
double[] singleDouble = ArraysKt.spreadOnlyDouble(packedDouble);
require(singleDouble != packedDouble && Arrays.equals(singleDouble, packedDouble));
require(Arrays.equals(compatibility.unsupported.UnsupportedDoubleArrayKt.arrays(), new double[] {1.0, 2.0}));
require(TypeUseCallsKt.typeNonnull(valuesDouble) == valuesDouble);
require(TypeUseCallsKt.typeNullable(null) == null);
require(TypeUseCallsKt.typeText("text").equals("text") && TypeUseCallsKt.typeNullableText(null) == null);
require(TypeUseCallsKt.typeCombined(valuesDouble) == valuesDouble && TypeUseCallsKt.typeCombinedNullable(null) == null);
require(ArraysKt.framedLong(true)[1] == 7L && ArraysKt.framedLong(false)[1] == 0L);
require(ArraysKt.framedBoolean(true)[1] == true && ArraysKt.framedBoolean(false)[1] == false);
require(ArraysKt.framedFloat(true)[1] == 7.5f && ArraysKt.framedFloat(false)[1] == 0.0f);
require(ArraysKt.framedDouble(true)[1] == 7.5 && ArraysKt.framedDouble(false)[1] == 0.0);
System.out.println("checks=" + checks);
}
}
