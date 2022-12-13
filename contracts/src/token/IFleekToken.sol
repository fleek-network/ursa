pragma solidity ^0.8.10;

interface IFleekToken {
    event Approval(address indexed owner, address indexed spender, uint256 amount);
    event MinterAdded(address indexed account);
    event MinterRemoved(address indexed account);
    event NewController(address indexed oldController, address indexed newController);
    event NewPendingController(address indexed controller, address indexed newController);
    event Transfer(address indexed from, address indexed to, uint256 amount);

    function DOMAIN_SEPARATOR() external view returns (bytes32);
    function acceptTransfer() external;
    function addMinter(address _account) external;
    function allowance(address, address) external view returns (uint256);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address) external view returns (uint256);
    function controller() external view returns (address);
    function decimals() external view returns (uint8);
    function name() external view returns (string memory);
    function nonces(address) external view returns (uint256);
    function pendingController() external view returns (address);
    function permit(address owner, address spender, uint256 value, uint256 deadline, uint8 v, bytes32 r, bytes32 s)
        external;
    function removeMinter(address _account) external;
    function symbol() external view returns (string memory);
    function totalSupply() external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
    function transferController(address _newController) external;
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}
