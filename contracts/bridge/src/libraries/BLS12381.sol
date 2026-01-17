// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title BLS12381
/// @notice Library for BLS12-381 signature verification using EIP-2537 precompiles
/// @dev Uses pairing check: e(signature, G2_generator) == e(H(message), pubkey)
library BLS12381 {
    // EIP-2537 precompile addresses
    address internal constant BLS12_G1ADD = address(0x0b);
    address internal constant BLS12_G1MUL = address(0x0c);
    address internal constant BLS12_G1MSM = address(0x0d);
    address internal constant BLS12_G2ADD = address(0x0e);
    address internal constant BLS12_G2MUL = address(0x0f);
    address internal constant BLS12_G2MSM = address(0x10);
    address internal constant BLS12_PAIRING = address(0x11);
    address internal constant BLS12_MAP_FP_TO_G1 = address(0x12);
    address internal constant BLS12_MAP_FP2_TO_G2 = address(0x13);

    // G1 point size (uncompressed): 128 bytes (2 x 64-byte coordinates)
    uint256 internal constant G1_POINT_SIZE = 128;
    // G2 point size (uncompressed): 256 bytes (2 x 128-byte coordinates, each coordinate is 2 x 64 bytes)
    uint256 internal constant G2_POINT_SIZE = 256;
    // Field element size: 64 bytes (padded to 64 for BLS12-381 in EIP-2537)
    uint256 internal constant FP_SIZE = 64;

    // G2 generator point (uncompressed, big-endian)
    // This is the standard G2 generator for BLS12-381
    bytes internal constant G2_GENERATOR = hex"00000000000000000000000000000000024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8"
        hex"0000000000000000000000000000000013e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e"
        hex"00000000000000000000000000000000ce5d527727d6e118cc9cdc6da2e351aadfd9baa8cbdd3a76d429a695160d12c923ac9cc3baca289e193548608b828010"
        hex"000000000000000000000000000000000606c4a02ea734cc32acd2b02bc28b99cb3e287e85a763af267492ab572e99ab3f370d275cec1da1aaa9075ff05f79be";

    error BLSPairingFailed();
    error BLSInvalidSignatureLength();
    error BLSInvalidPublicKeyLength();
    error BLSPrecompileCallFailed();
    error BLSHashToG1Failed();

    /// @notice Verify a BLS signature using the pairing check
    /// @param signature The BLS signature (G1 point, 128 bytes uncompressed)
    /// @param pubkey The BLS public key (G2 point, 256 bytes uncompressed)
    /// @param messageHash The message hash to verify (will be mapped to G1)
    /// @return valid True if the signature is valid
    function verify(bytes memory signature, bytes memory pubkey, bytes32 messageHash) internal view returns (bool valid) {
        if (signature.length != G1_POINT_SIZE) revert BLSInvalidSignatureLength();
        if (pubkey.length != G2_POINT_SIZE) revert BLSInvalidPublicKeyLength();

        // Map message hash to G1 point
        bytes memory hashedMessage = hashToG1(messageHash);

        // Pairing check: e(signature, G2_generator) == e(H(message), pubkey)
        // Reformulated as: e(signature, G2_generator) * e(-H(message), pubkey) == 1
        // Or: e(signature, G2_generator) * e(H(message), -pubkey) == 1
        // We use: pairing([sig, -H(m)], [G2_gen, pubkey]) == 1

        // Negate the hashed message (negate y-coordinate)
        bytes memory negHashedMessage = negateG1(hashedMessage);

        // Build pairing input: [sig || G2_gen || negHashedMessage || pubkey]
        bytes memory pairingInput = abi.encodePacked(
            signature,
            G2_GENERATOR,
            negHashedMessage,
            pubkey
        );

        // Call pairing precompile
        (bool success, bytes memory result) = BLS12_PAIRING.staticcall(pairingInput);
        if (!success || result.length != 32) revert BLSPrecompileCallFailed();

        // Result is 1 if pairing check passed
        valid = abi.decode(result, (uint256)) == 1;
    }

    /// @notice Map a bytes32 hash to a G1 point using the hash-to-curve precompile
    /// @param messageHash The hash to map
    /// @return g1Point The resulting G1 point (128 bytes)
    function hashToG1(bytes32 messageHash) internal view returns (bytes memory g1Point) {
        // Expand hash to field element (64 bytes, zero-padded on left)
        bytes memory fpElement = new bytes(FP_SIZE);
        // Place the 32-byte hash in the last 32 bytes (big-endian, zero-padded)
        assembly {
            mstore(add(fpElement, 64), messageHash)
        }

        // Call MAP_FP_TO_G1 precompile
        (bool success, bytes memory result) = BLS12_MAP_FP_TO_G1.staticcall(fpElement);
        if (!success || result.length != G1_POINT_SIZE) revert BLSHashToG1Failed();

        return result;
    }

    /// @notice Negate a G1 point (negate the y-coordinate)
    /// @param point The G1 point to negate (128 bytes)
    /// @return negated The negated G1 point
    function negateG1(bytes memory point) internal pure returns (bytes memory negated) {
        require(point.length == G1_POINT_SIZE, "Invalid G1 point length");

        // BLS12-381 field modulus p
        // p = 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab
        bytes memory p = hex"000000000000000000000000000000001a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab";

        negated = new bytes(G1_POINT_SIZE);

        // Copy x-coordinate (first 64 bytes)
        for (uint256 i = 0; i < FP_SIZE; i++) {
            negated[i] = point[i];
        }

        // Negate y-coordinate: y' = p - y
        // Extract y from point (bytes 64-127)
        bytes memory y = new bytes(FP_SIZE);
        for (uint256 i = 0; i < FP_SIZE; i++) {
            y[i] = point[FP_SIZE + i];
        }

        // Compute p - y using big integer subtraction
        bytes memory negY = subtractMod(p, y);
        for (uint256 i = 0; i < FP_SIZE; i++) {
            negated[FP_SIZE + i] = negY[i];
        }
    }

    /// @notice Subtract two 64-byte big integers (a - b) assuming a >= b
    /// @param a The minuend
    /// @param b The subtrahend
    /// @return result a - b
    function subtractMod(bytes memory a, bytes memory b) internal pure returns (bytes memory result) {
        require(a.length == FP_SIZE && b.length == FP_SIZE, "Invalid operand length");

        result = new bytes(FP_SIZE);
        int16 borrow = 0;

        // Subtract byte by byte from right to left (big-endian)
        for (uint256 i = FP_SIZE; i > 0; i--) {
            int16 diff = int16(uint16(uint8(a[i - 1]))) - int16(uint16(uint8(b[i - 1]))) - borrow;
            if (diff < 0) {
                diff += 256;
                borrow = 1;
            } else {
                borrow = 0;
            }
            result[i - 1] = bytes1(uint8(uint16(diff)));
        }
    }

    /// @notice Add two G1 points
    /// @param p1 First G1 point (128 bytes)
    /// @param p2 Second G1 point (128 bytes)
    /// @return sum The sum of the two points
    function g1Add(bytes memory p1, bytes memory p2) internal view returns (bytes memory sum) {
        require(p1.length == G1_POINT_SIZE && p2.length == G1_POINT_SIZE, "Invalid G1 point length");

        bytes memory input = abi.encodePacked(p1, p2);
        (bool success, bytes memory result) = BLS12_G1ADD.staticcall(input);
        if (!success || result.length != G1_POINT_SIZE) revert BLSPrecompileCallFailed();

        return result;
    }

    /// @notice Multiply a G1 point by a scalar
    /// @param point The G1 point (128 bytes)
    /// @param scalar The scalar (32 bytes)
    /// @return product The scalar multiplication result
    function g1Mul(bytes memory point, bytes32 scalar) internal view returns (bytes memory product) {
        require(point.length == G1_POINT_SIZE, "Invalid G1 point length");

        bytes memory input = abi.encodePacked(point, scalar);
        (bool success, bytes memory result) = BLS12_G1MUL.staticcall(input);
        if (!success || result.length != G1_POINT_SIZE) revert BLSPrecompileCallFailed();

        return result;
    }

    /// @notice Add two G2 points
    /// @param p1 First G2 point (256 bytes)
    /// @param p2 Second G2 point (256 bytes)
    /// @return sum The sum of the two points
    function g2Add(bytes memory p1, bytes memory p2) internal view returns (bytes memory sum) {
        require(p1.length == G2_POINT_SIZE && p2.length == G2_POINT_SIZE, "Invalid G2 point length");

        bytes memory input = abi.encodePacked(p1, p2);
        (bool success, bytes memory result) = BLS12_G2ADD.staticcall(input);
        if (!success || result.length != G2_POINT_SIZE) revert BLSPrecompileCallFailed();

        return result;
    }

    /// @notice Check if BLS precompiles are available
    /// @return available True if precompiles are available
    function precompilesAvailable() internal view returns (bool available) {
        // Try calling G1ADD with the identity element (point at infinity)
        // For BLS12-381, the point at infinity is all zeros
        bytes memory zeroPoint = new bytes(G1_POINT_SIZE);
        bytes memory input = abi.encodePacked(zeroPoint, zeroPoint);

        (bool success,) = BLS12_G1ADD.staticcall(input);
        return success;
    }
}
