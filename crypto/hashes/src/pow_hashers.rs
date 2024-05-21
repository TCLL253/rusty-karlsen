use crate::Hash;
use std::ops::BitXor;
use tiny_keccak::Hasher;

#[derive(Clone)]
pub struct PowB3Hash{
    pub hasher: blake3::Hasher,
}

#[derive(Clone)]
pub struct PowHash([u64; 25]);

#[derive(Clone)]
pub struct KHeavyHash;

#[derive(Clone)]
pub struct PowFishHash{
    // set the cache here not hasher
    pub context: Context,
}

const FNV_PRIME: u32 = 0x01000193;
const FULL_DATASET_ITEM_PARENTS: u32 = 512;
const NUM_DATASET_ACCESSES: i32 = 32;
const LIGHT_CACHE_ROUNDS: i32 = 3;

const LIGHT_CACHE_NUM_ITEMS: u32 = 1179641;
const FULL_DATASET_NUM_ITEMS: u32 = 37748717;
const SEED: Hash256 = Hash256([
    0xeb, 0x01, 0x63, 0xae, 0xf2, 0xab, 0x1c, 0x5a, 0x66, 0x31, 0x0c, 0x1c, 0x14, 0xd6, 0x0f, 0x42,
    0x55, 0xa9, 0xb3, 0x9b, 0x0e, 0xdf, 0x26, 0x53, 0x98, 0x44, 0xf1, 0x17, 0xad, 0x67, 0x21, 0x19,
]);

const SIZE_U32: usize = std::mem::size_of::<u32>();
const SIZE_U64: usize = std::mem::size_of::<u64>();


pub trait HashData {
    fn new() -> Self;
    fn as_bytes(&self) -> &[u8];
    fn as_bytes_mut(&mut self) -> &mut [u8];

    fn get_as_u32(&self, index: usize) -> u32 {
        u32::from_le_bytes(
            self.as_bytes()[index * SIZE_U32..index * SIZE_U32 + SIZE_U32]
                .try_into()
                .unwrap(),
        )
    }

    fn set_as_u32(&mut self, index: usize, value: u32) {
        self.as_bytes_mut()[index * SIZE_U32..index * SIZE_U32 + SIZE_U32]
            .copy_from_slice(&value.to_le_bytes())
    }

    fn get_as_u64(&self, index: usize) -> u64 {
        u64::from_le_bytes(
            self.as_bytes()[index * SIZE_U64..index * SIZE_U64 + SIZE_U64]
                .try_into()
                .unwrap(),
        )
    }

    fn set_as_u64(&mut self, index: usize, value: u64) {
        self.as_bytes_mut()[index * SIZE_U64..index * SIZE_U64 + SIZE_U64]
            .copy_from_slice(&value.to_le_bytes())
    }
}

#[derive(Debug)]
pub struct Hash256([u8; 32]);

impl HashData for Hash256 {
    fn new() -> Self {
        Self([0; 32])
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Hash512([u8; 64]);

impl HashData for Hash512 {
    fn new() -> Self {
        Self([0; 64])
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl BitXor<&Hash512> for &Hash512 {
    type Output = Hash512;

    fn bitxor(self, rhs: &Hash512) -> Self::Output {
        let mut hash = Hash512::new();

        for i in 0..64 {
            hash.0[i] = self.0[i] ^ rhs.0[i]
        }

        hash
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Hash1024([u8; 128]);

impl HashData for Hash1024 {
    fn new() -> Self {
        Self([0; 128])
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Hash1024 {
    fn from_512s(first: &Hash512, second: &Hash512) -> Self {
        let mut hash = Self::new();
        let (first_half, second_half) = hash.0.split_at_mut(first.0.len());
        first_half.copy_from_slice(&first.0);
        second_half.copy_from_slice(&second.0);

        hash
    }
}

#[derive(Clone)]
pub struct Context {
    pub light_cache: Box<[Hash512]>,
    pub full_dataset: Option<Box<[Hash1024]>>,
}

impl Context {
    pub fn new(full: bool) -> Self {
        // Vec into boxed sliced, because you can't allocate an array directly on
        // the heap in rust
        // https://stackoverflow.com/questions/25805174/creating-a-fixed-size-array-on-heap-in-rust/68122278#68122278
        let mut light_cache =
            vec![Hash512::new(); LIGHT_CACHE_NUM_ITEMS as usize].into_boxed_slice();
            Context::build_light_cache(&mut light_cache);

        let full_dataset = if full {
            Some(vec![Hash1024::new(); FULL_DATASET_NUM_ITEMS as usize].into_boxed_slice())
        } else {
            None
        };

        Context {
            light_cache,
            full_dataset,
        }
    }

    fn build_light_cache(cache: &mut [Hash512]) {
        let mut item: Hash512 = Hash512::new();
        PowFishHash::keccak(&mut item.0, &SEED.0);
        cache[0] = item;
    
        for cache_item in cache
            .iter_mut()
            .take(LIGHT_CACHE_NUM_ITEMS as usize)
            .skip(1)
        {
            PowFishHash::keccak_in_place(&mut item.0);
            *cache_item = item;
        }
    
        for _ in 0..LIGHT_CACHE_ROUNDS {
            for i in 0..LIGHT_CACHE_NUM_ITEMS {
                // First index: 4 first bytes of the item as little-endian integer
                let t: u32 = cache[i as usize].get_as_u32(0);
                let v: u32 = t % LIGHT_CACHE_NUM_ITEMS;
    
                // Second index
                let w: u32 =
                    (LIGHT_CACHE_NUM_ITEMS.wrapping_add(i.wrapping_sub(1))) % LIGHT_CACHE_NUM_ITEMS;
    
                let x = &cache[v as usize] ^ &cache[w as usize];
                PowFishHash::keccak(&mut cache[i as usize].0, &x.0);
            }
        }
    }
}

impl PowFishHash {


    #[inline]
    pub fn new() -> Self {
        let mut context = Context::new(false);
        Self { context }
    }

    pub fn keccak(out: &mut [u8], data: &[u8]) {
        let mut hasher = tiny_keccak::Keccak::v512();
        hasher.update(data);
        hasher.finalize(out);
    }

    fn keccak_in_place(data: &mut [u8]) {
        //TODO remove tiny_keccak with asm keccak
        let mut hasher = tiny_keccak::Keccak::v512();
        hasher.update(data);
        hasher.finalize(data);
    }

    fn fnv1(u: u32, v: u32) -> u32 {
        (u * FNV_PRIME) ^ v
    }
    
    fn fnv1_512(u: Hash512, v: Hash512) -> Hash512 {
        let mut r = Hash512::new();
    
        for i in 0..r.0.len() / SIZE_U32 {
            r.set_as_u32(i, PowFishHash::fnv1(u.get_as_u32(i), v.get_as_u32(i)));
        }
    
        r
    }

    fn calculate_dataset_item_1024(light_cache: &[Hash512], index: usize) -> Hash1024 {
        let seed0 = (index * 2) as u32;
        let seed1 = seed0 + 1;
    
        let mut mix0 = light_cache[(seed0 % LIGHT_CACHE_NUM_ITEMS) as usize];
        let mut mix1 = light_cache[(seed1 % LIGHT_CACHE_NUM_ITEMS) as usize];
    
        let mix0_seed = mix0.get_as_u32(0) ^ seed0;
        let mix1_seed = mix1.get_as_u32(0) ^ seed1;
    
        mix0.set_as_u32(0, mix0_seed);
        mix1.set_as_u32(0, mix1_seed);
    
        PowFishHash::keccak_in_place(&mut mix0.0);
        PowFishHash::keccak_in_place(&mut mix1.0);
    
        let num_words: u32 = (std::mem::size_of_val(&mix0) / SIZE_U32) as u32;
        for j in 0..FULL_DATASET_ITEM_PARENTS {
            let t0 = PowFishHash::fnv1(seed0 ^ j, mix0.get_as_u32((j % num_words) as usize));
            let t1 = PowFishHash::fnv1(seed1 ^ j, mix1.get_as_u32((j % num_words) as usize));
            mix0 = PowFishHash::fnv1_512(mix0, light_cache[(t0 % LIGHT_CACHE_NUM_ITEMS) as usize]);
            mix1 = PowFishHash::fnv1_512(mix1, light_cache[(t1 % LIGHT_CACHE_NUM_ITEMS) as usize]);
        }
    
        PowFishHash::keccak_in_place(&mut mix0.0);
        PowFishHash::keccak_in_place(&mut mix1.0);
    
        Hash1024::from_512s(&mix0, &mix1)
    }

    fn lookup(context: &mut Context, index: usize) -> Hash1024 {
        match &mut context.full_dataset {
            Some(dataset) => {
                let item = &mut dataset[index];
                if item.get_as_u64(0) == 0 {
                    *item = PowFishHash::calculate_dataset_item_1024(&context.light_cache, index);
                }
    
                *item
            }
            None => PowFishHash::calculate_dataset_item_1024(&context.light_cache, index),
        }
    }
    

    fn fishhash_kernel(context: &mut Context, seed: &Hash512) -> Hash256 {
        let mut mix = Hash1024::from_512s(seed, seed);
    
        for _ in 0..NUM_DATASET_ACCESSES as usize {
            // Calculate new fetching indexes
            let p0 = mix.get_as_u32(0) % FULL_DATASET_NUM_ITEMS;
            let p1 = mix.get_as_u32(4) % FULL_DATASET_NUM_ITEMS;
            let p2 = mix.get_as_u32(8) % FULL_DATASET_NUM_ITEMS;
    
            let fetch0 = PowFishHash::lookup(context, p0 as usize);
            let mut fetch1 = PowFishHash::lookup(context, p1 as usize);
            let mut fetch2 = PowFishHash::lookup(context, p2 as usize);
    
            // Modify fetch1 and fetch2
            for j in 0..32 {
                fetch1.set_as_u32(j, PowFishHash::fnv1(mix.get_as_u32(j), fetch1.get_as_u32(j)));
                fetch2.set_as_u32(j, mix.get_as_u32(j) ^ fetch2.get_as_u32(j));
            }
    
            // Final computation of new mix
            for j in 0..16 {
                mix.set_as_u64(
                    j,
                    fetch0.get_as_u64(j) * fetch1.get_as_u64(j) + fetch2.get_as_u64(j),
                );
            }
        }
    
        // Collapse the result into 32 bytes
        let mut mix_hash = Hash256::new();
        let num_words = std::mem::size_of_val(&mix) / SIZE_U32;
    
        for i in (0..num_words).step_by(4) {
            let h1 = PowFishHash::fnv1(mix.get_as_u32(i), mix.get_as_u32(i + 1));
            let h2 = PowFishHash::fnv1(h1, mix.get_as_u32(i + 2));
            let h3 = PowFishHash::fnv1(h2, mix.get_as_u32(i + 3));
            mix_hash.set_as_u32(i / 4, h3);
        }
    
        mix_hash
    }

    #[inline]
    pub fn hash(output: &mut [u8], context: &mut Context, header: &[u8]) {
        let mut seed: Hash512 = Hash512::new();
    
        let mut hasher = blake3::Hasher::new();
        hasher.update(header);
        let mut output_reader = hasher.finalize_xof();
        output_reader.fill(&mut seed.0);
    
        let mix_hash = PowFishHash::fishhash_kernel(context, &seed);
    
        let mut final_data: [u8; 96] = [0; 96];
        final_data[0..64].copy_from_slice(&seed.0);
        final_data[64..].copy_from_slice(&mix_hash.0);
    
        let hash = blake3::hash(&final_data);
        output.copy_from_slice(hash.as_bytes());
    }


}

impl PowB3Hash {

    #[inline]
    pub fn new(pre_pow_hash: Hash, timestamp: u64) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&pre_pow_hash.as_bytes());
        hasher.update(&timestamp.to_le_bytes());
        let array: [u8; 32] = [0; 32];
        hasher.update(&array);
        Self { hasher }
    }

    

    

    #[inline(always)]
    pub fn finalize_with_nonce(mut self, nonce: u64) -> Hash {
        
        self.hasher.update(&nonce.to_le_bytes());
        let hash = self.hasher.finalize();
        Hash(*hash.as_bytes())

        //Hash::from_le_u64(self.0[..4].try_into().unwrap())
    }

}

impl PowHash {
    // The initial state of `cSHAKE256("ProofOfWorkHash")`
    // [10] -> 1123092876221303310 ^ 0x04(padding byte) = 1123092876221303306
    // [16] -> 10306167911662716186 ^ 0x8000000000000000(final padding) = 1082795874807940378
    #[rustfmt::skip]
    const INITIAL_STATE: [u64; 25] = [
        1242148031264380989, 3008272977830772284, 2188519011337848018, 1992179434288343456, 8876506674959887717,
        5399642050693751366, 1745875063082670864, 8605242046444978844, 17936695144567157056, 3343109343542796272,
        1123092876221303306, 4963925045340115282, 17037383077651887893, 16629644495023626889, 12833675776649114147,
        3784524041015224902, 1082795874807940378, 13952716920571277634, 13411128033953605860, 15060696040649351053,
        9928834659948351306, 5237849264682708699, 12825353012139217522, 6706187291358897596, 196324915476054915,
    ];
    #[inline]
    pub fn new(pre_pow_hash: Hash, timestamp: u64) -> Self {
        let mut start = Self::INITIAL_STATE;
        for (pre_pow_word, state_word) in pre_pow_hash.iter_le_u64().zip(start.iter_mut()) {
            *state_word ^= pre_pow_word;
        }
        start[4] ^= timestamp;
        Self(start)
    }

    

    

    #[inline(always)]
    pub fn finalize_with_nonce(mut self, nonce: u64) -> Hash {
        
        self.0[9] ^= nonce;


        /*
        let pre_pow_hash = Hash([42; 32]);
        println!("pre_pow_hash : {:?}", pre_pow_hash);
        let mut new_hash = b3::convert_u8_to_u64_array(pre_pow_hash.0);
        println!("new_hash : {:?}", new_hash);
        println!("Hash(new_hash) : {:?}", Hash::from_le_u64(new_hash[..4].try_into().unwrap()));
        //keccak256::f1600(&mut new_hash);
        b3::blake3_hash(&mut new_hash);
        println!("Hash(new_hash) : {:?}", Hash::from_le_u64(new_hash[..4].try_into().unwrap()));
        */

        

        //keccak256::f1600(&mut self.0);
        //b3::blake3_hash_u64_array(&mut self.0);
        //println!("self : {:?}", self.0);
        b3::blake3_hash(&mut self.0);

        Hash::from_le_u64(self.0[..4].try_into().unwrap())
    }

    #[inline(always)]
    pub fn test_hash(state: &mut [u64; 25]) {
        //b3::blake3_hash(state);
        keccak256::f1600(state);
    }

}

impl KHeavyHash {
    // The initial state of `cSHAKE256("HeavyHash")`
    // [4] -> 16654558671554924254 ^ 0x04(padding byte) = 16654558671554924250
    // [16] -> 9793466274154320918 ^ 0x8000000000000000(final padding) = 570094237299545110
    #[rustfmt::skip]
    const INITIAL_STATE: [u64; 25] = [
        4239941492252378377, 8746723911537738262, 8796936657246353646, 1272090201925444760, 16654558671554924250,
        8270816933120786537, 13907396207649043898, 6782861118970774626, 9239690602118867528, 11582319943599406348,
        17596056728278508070, 15212962468105129023, 7812475424661425213, 3370482334374859748, 5690099369266491460,
        8596393687355028144, 570094237299545110, 9119540418498120711, 16901969272480492857, 13372017233735502424,
        14372891883993151831, 5171152063242093102, 10573107899694386186, 6096431547456407061, 1592359455985097269,
    ];
    #[inline]
    pub fn hash(in_hash: Hash) -> Hash {
        let mut state = Self::INITIAL_STATE;
        for (pre_pow_word, state_word) in in_hash.iter_le_u64().zip(state.iter_mut()) {
            *state_word ^= pre_pow_word;
        }
        keccak256::f1600(&mut state);
        //b3::blake3_hash_u64_array(&mut state);
        Hash::from_le_u64(state[..4].try_into().unwrap())
    }
}

mod b3 {

    pub fn convert_u8_to_u64_array(bytes: [u8; 32]) -> [u64; 25] {
        let mut u64_array = [0u64; 25];
    
        // Fill the u64_array using data from bytes
        for (i, chunk) in bytes.chunks_exact(8).enumerate() {
            let mut u64_val = 0u64;
    
            for (j, &b) in chunk.iter().enumerate() {
                u64_val |= (b as u64) << (j * 8);
            }
    
            u64_array[i] = u64_val;
        }
    
        // The remaining elements in the u64_array are already zero-filled
        u64_array
    }

    /*
    pub fn blake3_hash(state: &mut [u64; 25]) {
        // Create a new Blake3 hasher
        let mut hasher = blake3::Hasher::new();

        // Update the hasher with the bytes of the array
        for &num in state {
            hasher.update(&num.to_le_bytes());
        }

        // Finalize the hash to obtain a 32-byte array
        let result = hasher.finalize();

    }
    */

    pub fn blake3_hash(state: &mut [u64; 25]) {
        // Interpret the u64 state array as bytes for hashing
        let state_as_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                state.as_ptr() as *const u8,
                state.len() * std::mem::size_of::<u64>(),
            )
        };
    
        // Compute the BLAKE3 hash of the state bytes
        let hash = blake3::hash(state_as_bytes);
    
        // Convert the hash output (32 bytes) into u64 and store back into the first 4 elements
        let hash_bytes = hash.as_bytes();
        for i in 0..4 {
            state[i] = u64::from_le_bytes([
                hash_bytes[i * 8 + 0],
                hash_bytes[i * 8 + 1],
                hash_bytes[i * 8 + 2],
                hash_bytes[i * 8 + 3],
                hash_bytes[i * 8 + 4],
                hash_bytes[i * 8 + 5],
                hash_bytes[i * 8 + 6],
                hash_bytes[i * 8 + 7],
            ]);
        }
    
        // Optional: Zero out remaining parts of the state, or maintain their original values
        for i in 4..25 {
            state[i] = 0;
        }
    }

    pub(super) fn blake3_hash_u64_array(input: &mut [u64; 25]) -> &mut [u64; 25] {
        // Convert the `[u64; 25]` array to a byte slice
        let byte_slice: &[u8] = unsafe {
            std::slice::from_raw_parts(
                input.as_ptr() as *const u8,
                std::mem::size_of::<[u64; 25]>()
            )
        };
    
        // Compute the Blake3 hash of the input byte slice
        let hash = blake3::hash(byte_slice);
    
        // Copy the hash output back into the original `[u64; 25]` array
        // We will only copy as much data as can fit into the array
        let mut hash_bytes = [0u8; std::mem::size_of::<[u64; 25]>()];
        let hash_slice = hash.as_bytes();
        let copy_length = std::cmp::min(hash_bytes.len(), hash_slice.len());
        hash_bytes[..copy_length].copy_from_slice(&hash_slice[..copy_length]);
    
        // Now convert the bytes back to u64 chunks and modify the original array
        let new_u64_slice: &[u64] = unsafe {
            std::slice::from_raw_parts(
                hash_bytes.as_ptr() as *const u64,
                hash_bytes.len() / std::mem::size_of::<u64>()
            )
        };
    
        for (target, &src) in input.iter_mut().zip(new_u64_slice.iter()) {
            *target = src;
        }
    
        // Return the modified input array
        input
    }

}

mod keccak256 {
    #[cfg(any(not(target_arch = "x86_64"), feature = "no-asm", target_os = "windows"))]
    #[inline(always)]
    pub(super) fn f1600(state: &mut [u64; 25]) {
        keccak::f1600(state);
    }

    #[cfg(all(target_arch = "x86_64", not(feature = "no-asm"), not(target_os = "windows")))]
    #[inline(always)]
    pub(super) fn f1600(state: &mut [u64; 25]) {
        extern "C" {
            fn KeccakF1600(state: &mut [u64; 25]);
        }
        unsafe { KeccakF1600(state) }
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::null;

    use super::{KHeavyHash, PowHash};
    use crate::Hash;
    use sha3::digest::{ExtendableOutput, Update, XofReader};
    use sha3::{CShake256, CShake256Core};

    const PROOF_OF_WORK_DOMAIN: &[u8] = b"ProofOfWorkHash";
    const HEAVY_HASH_DOMAIN: &[u8] = b"HeavyHash";

    #[test]
    fn test_pow_hash() {

        let pre_pow_hash = Hash([42; 32]);

        let timestamp: u64 = 5435345234;
        let nonce: u64 = 432432432;
        let pre_pow_hash = Hash([42; 32]);

        /*
        let initial_bytes = [0xC1, 0xEC, 0xFD, 0xFC]; // Define the starting byte values
        let mut full_array = [0; 32]; // Create an array initialized to zeros
        full_array[..initial_bytes.len()].copy_from_slice(&initial_bytes);
        let pre_pow_hash2 = Hash(full_array);
        */




        let hasher = PowHash::new(pre_pow_hash, timestamp);
        let hash1 = hasher.finalize_with_nonce(nonce);

        let hasher = CShake256::from_core(CShake256Core::new(PROOF_OF_WORK_DOMAIN))
            .chain(pre_pow_hash.0)
            .chain(timestamp.to_le_bytes())
            .chain([0u8; 32])
            .chain(nonce.to_le_bytes());
        let mut hash2 = [0u8; 32];
        hasher.finalize_xof().read(&mut hash2);
        //println!("init : {:?}", initial_bytes);
        //println!("full : {:?}", full_array);
        println!("hash1 : {:?}", hash1);
        println!("Hash(hash2) : {:?}", Hash(hash2));
        //println!("pow : {:?}", PowHash::test_hash(full_array));
        assert_eq!(Hash(hash2), hash1);
        
    }

    #[test]
    fn test_heavy_hash() {
        let val = Hash([42; 32]);
        let hash1 = KHeavyHash::hash(val);

        let hasher = CShake256::from_core(CShake256Core::new(HEAVY_HASH_DOMAIN)).chain(val.0);
        let mut hash2 = [0u8; 32];
        hasher.finalize_xof().read(&mut hash2);
        assert_eq!(Hash(hash2), hash1);
    }
}
