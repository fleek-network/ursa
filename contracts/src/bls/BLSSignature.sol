// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;


//This contract is an example of verifying an aggregated BLS signature
//It uses the precompiled BLS12-381 contract from EIP-2537 at 0x10 (https://eips.ethereum.org/EIPS/eip-2537)
//This contract is not live on mainnet but the idea is to get whatever L2 we use to deploy the precompile for us
//This should not be too hard of a sell because the code for the precompile is finished, audited and merged into the geth codebase.
contract BLSSignature {
    uint256 constant PUBLIC_KEY_LENGTH = 48;
    uint256 constant SIGNATURE_LENGTH = 96;
    uint256 constant WITHDRAWAL_CREDENTIALS_LENGTH = 32;
    uint256 constant WEI_PER_GWEI = 1e9;

    uint8 constant BLS12_381_PAIRING_PRECOMPILE_ADDRESS = 0x10;
    uint8 constant BLS12_381_MAP_FIELD_TO_CURVE_PRECOMPILE_ADDRESS = 0x12;
    uint8 constant BLS12_381_G2_ADD_ADDRESS = 0xD;
    string constant BLS_SIG_DST = "BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_+";
    bytes1 constant BLS_BYTE_WITHOUT_FLAGS_MASK = bytes1(0x1f);

    uint8 constant MOD_EXP_PRECOMPILE_ADDRESS = 0x5;

    // Fp is a field element with the high-order part stored in `a`.
    struct Fp {
        uint a;
        uint b;
    }

    // Fp2 is an extension field element with the coefficient of the
    // quadratic non-residue stored in `b`, i.e. p = a + i * b
    struct Fp2 {
        Fp a;
        Fp b;
    }

    // G1Point represents a point on BLS12-381 over Fp with coordinates (X,Y);
    struct G1Point {
        Fp X;
        Fp Y;
    }

    // G2Point represents a point on BLS12-381 over Fp2 with coordinates (X,Y);
    struct G2Point {
        Fp2 X;
        Fp2 Y;
    }


}