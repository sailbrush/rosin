#![allow(clippy::disallowed_types)]

use std::{borrow::Cow, cell::*, collections::*, marker::PhantomData, num::*, ops::Range, path::*, rc::Rc, sync::atomic::*, sync::*, time::*};

use crate::parking_lot;
use crate::prelude::*;

pub static DEFAULT_DEPTH: u64 = 64;

/// Used by the hot-reload feature to determine if the in-memory layout of a type has changed between compiles.
///
/// This is for use during development only, and is likely unsound.
///
/// Primitive types return a hard coded random number: https://xkcd.com/221/
///
/// Standard wrapper types mutate the TypeHash of their internal type by adding an odd number, and XORing with an even number.
/// This is guaranteed to permute the values in a single-cycle derangement.
///
/// The depth parameter stops self-referential types from overflowing the stack.
pub trait TypeHash {
    fn get_typehash(depth: u64) -> u64;
}

impl TypeHash for Stylesheet {
    fn get_typehash(_: u64) -> u64 {
        797117776116410908
    }
}

impl TypeHash for NodeId {
    fn get_typehash(_: u64) -> u64 {
        5684255684596756112
    }
}

impl TypeHash for Range<u64> {
    fn get_typehash(_: u64) -> u64 {
        17240480695858487657
    }
}

impl TypeHash for i8 {
    fn get_typehash(_: u64) -> u64 {
        13299827456864925276
    }
}

impl TypeHash for i16 {
    fn get_typehash(_: u64) -> u64 {
        12147770752965991077
    }
}

impl TypeHash for i32 {
    fn get_typehash(_: u64) -> u64 {
        3943228079724900559
    }
}

impl TypeHash for i64 {
    fn get_typehash(_: u64) -> u64 {
        64829118325855806
    }
}

impl TypeHash for i128 {
    fn get_typehash(_: u64) -> u64 {
        1753877801635960860
    }
}

impl TypeHash for isize {
    fn get_typehash(_: u64) -> u64 {
        18050050955313131343
    }
}

impl TypeHash for u8 {
    fn get_typehash(_: u64) -> u64 {
        9823229221207696132
    }
}

impl TypeHash for u16 {
    fn get_typehash(_: u64) -> u64 {
        10658431189036031640
    }
}

impl TypeHash for u32 {
    fn get_typehash(_: u64) -> u64 {
        10925752636944610817
    }
}

impl TypeHash for u64 {
    fn get_typehash(_: u64) -> u64 {
        10825815970367267140
    }
}

impl TypeHash for u128 {
    fn get_typehash(_: u64) -> u64 {
        13288942628773597616
    }
}

impl TypeHash for usize {
    fn get_typehash(_: u64) -> u64 {
        14167080105320939518
    }
}

impl TypeHash for f32 {
    fn get_typehash(_: u64) -> u64 {
        14000545469524009037
    }
}

impl TypeHash for f64 {
    fn get_typehash(_: u64) -> u64 {
        13780367636362361805
    }
}

impl TypeHash for char {
    fn get_typehash(_: u64) -> u64 {
        341514802630720643
    }
}

impl TypeHash for bool {
    fn get_typehash(_: u64) -> u64 {
        558085288902155710
    }
}

impl TypeHash for NonZeroI8 {
    fn get_typehash(_: u64) -> u64 {
        2604490389784826050
    }
}

impl TypeHash for NonZeroI16 {
    fn get_typehash(_: u64) -> u64 {
        11548639026342739202
    }
}

impl TypeHash for NonZeroI32 {
    fn get_typehash(_: u64) -> u64 {
        15681202112120304226
    }
}

impl TypeHash for NonZeroI64 {
    fn get_typehash(_: u64) -> u64 {
        16586206504776851466
    }
}

impl TypeHash for NonZeroI128 {
    fn get_typehash(_: u64) -> u64 {
        12002769979917583625
    }
}

impl TypeHash for NonZeroIsize {
    fn get_typehash(_: u64) -> u64 {
        1707390894698746880
    }
}

impl TypeHash for NonZeroU8 {
    fn get_typehash(_: u64) -> u64 {
        3216790058810511573
    }
}

impl TypeHash for NonZeroU16 {
    fn get_typehash(_: u64) -> u64 {
        15093403964989460326
    }
}

impl TypeHash for NonZeroU32 {
    fn get_typehash(_: u64) -> u64 {
        14285792637719858337
    }
}

impl TypeHash for NonZeroU64 {
    fn get_typehash(_: u64) -> u64 {
        13718507436580098965
    }
}

impl TypeHash for NonZeroU128 {
    fn get_typehash(_: u64) -> u64 {
        16015023350597354814
    }
}

impl TypeHash for NonZeroUsize {
    fn get_typehash(_: u64) -> u64 {
        14961711578572294199
    }
}

impl TypeHash for AtomicBool {
    fn get_typehash(_: u64) -> u64 {
        17314495795645180008
    }
}

impl TypeHash for AtomicI8 {
    fn get_typehash(_: u64) -> u64 {
        4720485528609364652
    }
}

impl TypeHash for AtomicI16 {
    fn get_typehash(_: u64) -> u64 {
        16869602587274001595
    }
}

impl TypeHash for AtomicI32 {
    fn get_typehash(_: u64) -> u64 {
        6799103294891429625
    }
}

impl TypeHash for AtomicI64 {
    fn get_typehash(_: u64) -> u64 {
        16912318281116356429
    }
}

impl TypeHash for AtomicIsize {
    fn get_typehash(_: u64) -> u64 {
        4342704652612741541
    }
}

impl TypeHash for AtomicU8 {
    fn get_typehash(_: u64) -> u64 {
        9177298053178097885
    }
}

impl TypeHash for AtomicU16 {
    fn get_typehash(_: u64) -> u64 {
        2011692480643740783
    }
}

impl TypeHash for AtomicU32 {
    fn get_typehash(_: u64) -> u64 {
        13832464205748323888
    }
}

impl TypeHash for AtomicU64 {
    fn get_typehash(_: u64) -> u64 {
        9475688357472807699
    }
}

impl TypeHash for AtomicUsize {
    fn get_typehash(_: u64) -> u64 {
        640042871632520671
    }
}

impl TypeHash for &str {
    fn get_typehash(_: u64) -> u64 {
        1481073601717420376
    }
}

impl TypeHash for String {
    fn get_typehash(_: u64) -> u64 {
        18154154088871876003
    }
}

impl TypeHash for SystemTime {
    fn get_typehash(_: u64) -> u64 {
        14318265379624179536
    }
}

impl TypeHash for Duration {
    fn get_typehash(_: u64) -> u64 {
        11959697877109502370
    }
}

impl TypeHash for PathBuf {
    fn get_typehash(_: u64) -> u64 {
        9264597754931132995
    }
}

impl TypeHash for Path {
    fn get_typehash(_: u64) -> u64 {
        9337725397773126321
    }
}

impl<T: TypeHash> TypeHash for Option<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(691608541) ^ 10072782681291120786
    }
}

impl<T: TypeHash> TypeHash for Box<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(803559003) ^ 18274620969753463218
    }
}

impl<T: TypeHash> TypeHash for PhantomData<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(654832673) ^ 2411973491565164156
    }
}

impl<T: TypeHash> TypeHash for [T] {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(100290207) ^ 9448188587891344692
    }
}

impl<T: TypeHash> TypeHash for *const T {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(4515998741) ^ 7210121360167608524
    }
}

impl<T: TypeHash> TypeHash for *mut T {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(40778783405) ^ 10364067501822314436
    }
}

impl<T: TypeHash> TypeHash for &T {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(3132936747) ^ 9184528352377145842
    }
}

impl<T: TypeHash> TypeHash for &mut T {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(6618025647) ^ 8812502421895527568
    }
}

impl<T: TypeHash> TypeHash for Vec<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(270494513) ^ 12106089999529058752
    }
}

impl<T: TypeHash> TypeHash for VecDeque<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(1816538931) ^ 10550311512695415078
    }
}

impl<T: TypeHash> TypeHash for LinkedList<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(9941412909) ^ 7333782979616585078
    }
}

impl<T: TypeHash> TypeHash for HashSet<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(35479841777) ^ 15679579202962282488
    }
}

impl<T: TypeHash> TypeHash for BTreeSet<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(728257573) ^ 2354363518970195144
    }
}

impl<T: TypeHash> TypeHash for BinaryHeap<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(54960745571) ^ 9841879103948683828
    }
}

impl<T: TypeHash> TypeHash for Rc<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(226500229) ^ 1397709351770713004
    }
}

impl<T: TypeHash> TypeHash for Arc<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(416722401) ^ 15845837182876551926
    }
}

impl<T: TypeHash> TypeHash for RwLock<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(753649701) ^ 1643833027784986004
    }
}

impl<T: TypeHash> TypeHash for Mutex<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(297544501) ^ 5572703376962736786
    }
}

impl<T: TypeHash> TypeHash for parking_lot::RwLock<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(1687010153) ^ 15691310327119941682
    }
}

impl<T: TypeHash> TypeHash for parking_lot::Mutex<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(53435754679) ^ 4324940292681255142
    }
}

impl<T: TypeHash> TypeHash for Cell<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(937821685) ^ 18256803205591787098
    }
}

impl<T: TypeHash> TypeHash for RefCell<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(7482831623) ^ 12190553668240036944
    }
}

impl<T: TypeHash + Send + Sync> TypeHash for Var<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(3769343155) ^ 10513299626524530284
    }
}

impl<T: TypeHash + Send + Sync> TypeHash for WeakVar<T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(8871176915) ^ 9494732307598762156
    }
}

impl<T: TypeHash + Clone> TypeHash for Cow<'_, T> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        T::get_typehash(depth - 1).wrapping_add(4619336169) ^ 3145598195736926078
    }
}

impl<K: TypeHash, V: TypeHash> TypeHash for HashMap<K, V> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        K::get_typehash(depth - 1)
            .wrapping_add(V::get_typehash(depth - 1) << 1)
            .wrapping_add(1531269491)
            ^ 4180758651472523334
    }
}

impl<K: TypeHash, V: TypeHash> TypeHash for BTreeMap<K, V> {
    fn get_typehash(depth: u64) -> u64 {
        if depth == 0 {
            return 1;
        }
        K::get_typehash(depth - 1)
            .wrapping_add(V::get_typehash(depth - 1) << 1)
            .wrapping_add(90571190583)
            ^ 13082464548550978160
    }
}
