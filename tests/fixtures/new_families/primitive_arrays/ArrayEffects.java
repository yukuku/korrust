package newfamilies.arrays;
public final class ArrayEffects {
public static String trace = "";
public static boolean nullArray;
public static void reset(boolean empty) { trace = ""; nullArray = empty; }
public static int index() { trace += "I"; return 0; }
public static long[] longValues = new long[] {1L, 2L};
public static long[] longArray() { trace += "A"; return nullArray ? null : longValues; }
public static long longValue() { trace += "V"; return 77L; }
public static boolean[] booleanValues = new boolean[] {true, false};
public static boolean[] booleanArray() { trace += "A"; return nullArray ? null : booleanValues; }
public static boolean booleanValue() { trace += "V"; return true; }
public static float[] floatValues = new float[] {1.25f, 2.5f};
public static float[] floatArray() { trace += "A"; return nullArray ? null : floatValues; }
public static float floatValue() { trace += "V"; return 77.25f; }
public static double[] doubleValues = new double[] {1.25, 2.5};
public static double[] doubleArray() { trace += "A"; return nullArray ? null : doubleValues; }
public static double doubleValue() { trace += "V"; return 77.5; }
}
